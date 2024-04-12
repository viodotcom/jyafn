mod murmur;

use get_size::GetSize;
use hashbrown::HashMap;
use serde_derive::{Deserialize, Serialize};
use std::hash::{BuildHasher, Hasher};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;
use zip::read::ZipFile;

use crate::Error;

use super::layout::{Buffer, Layout};

#[derive(Debug, Default, Clone, Copy)]
pub struct UnHash;

impl BuildHasher for UnHash {
    type Hasher = UnHasher;
    fn build_hasher(&self) -> UnHasher {
        UnHasher::default()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnHasher(usize, [u8; 8]);

impl Hasher for UnHasher {
    fn finish(&self) -> u64 {
        u64::from_le_bytes(self.1)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.1[self.0 % 8] = b;
            self.0 += 1;
        }
    }
}

fn update_hash(hash: i64, value: i64) -> i64 {
    let hash = u64::from_ne_bytes(hash.to_ne_bytes());
    let value = u64::from_ne_bytes(value.to_ne_bytes());
    let updated = murmur::murmur_hash64a(&u64::to_le_bytes(value), hash);

    i64::from_ne_bytes(updated.to_ne_bytes())
}

fn hash(line: &Buffer) -> u64 {
    let mut hash = 0u64;

    for value in line.chunks(8) {
        hash = u64::from_ne_bytes(
            update_hash(
                i64::from_ne_bytes(hash.to_ne_bytes()),
                i64::from_ne_bytes(
                    value
                        .try_into()
                        .expect("size of buffer is always multiple of 8"),
                ),
            )
            .to_ne_bytes(),
        );
    }

    hash
}

#[typetag::serde(tag = "type")]
pub trait StorageType: std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe {
    fn init(&self) -> Result<Box<dyn Storage>, Error>;
    fn read(&self, f: ZipFile<'_>) -> Result<Box<dyn Storage>, Error>;
}

pub trait Storage: std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe {
    fn insert(&mut self, hash: u64, value: Buffer);
    fn get(&self, hash: u64) -> Option<&Buffer>;
    /// The ammount of heap used by this storage.
    fn size(&self) -> usize;
    fn dump(&self) -> Result<Vec<u8>, Error>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HashMapStorage;

#[typetag::serde]
impl StorageType for HashMapStorage {
    fn init(&self) -> Result<Box<dyn Storage>, Error> {
        Ok(Box::new(HashTable::default()))
    }

    fn read(&self, f: ZipFile<'_>) -> Result<Box<dyn Storage>, Error> {
        let map = bincode::deserialize_from(f).map_err(Error::Deserialization)?;
        Ok(Box::new(HashTable(map)))
    }
}

#[derive(Debug, Default)]
struct HashTable(HashMap<u64, Buffer, UnHash>);

impl Storage for HashTable {
    fn insert(&mut self, hash: u64, value: Buffer) {
        self.0.insert(hash, value);
    }

    fn get(&self, hash: u64) -> Option<&Buffer> {
        self.0.get(&hash)
    }

    fn size(&self) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of::<(u64, Buffer)>() * self.0.raw_table().capacity()
            + self
                .0
                .iter()
                .map(|(_, buf)| buf.get_heap_size())
                .sum::<usize>()
    }

    fn dump(&self) -> Result<Vec<u8>, Error> {
        Ok(bincode::serialize(&self.0).expect("serialization never fails"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mapping {
    key_layout: Layout,
    value_layout: Layout,
    storage_type: Arc<dyn StorageType>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[serde(default)]
    storage: Option<Box<dyn Storage>>,
    /// We need this field because we _hardcode_ the pointer to this struct in the
    /// function code. If this moves anywhere, we get the pleasure of accessing bad
    /// memory and The Most Horrible Things™ ensue.
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[serde(default = "new_pinned")]
    _pin: std::marker::PhantomPinned,
}

fn new_pinned() -> std::marker::PhantomPinned {
    std::marker::PhantomPinned
}

impl GetSize for Mapping {
    fn get_heap_size(&self) -> usize {
        if let Some(storage) = &self.storage {
            storage.size()
        } else {
            0
        }
    }
}

impl Mapping {
    pub(crate) fn new<S>(
        key_layout: Layout,
        value_layout: Layout,
        storage_type: S,
    ) -> Result<Mapping, Error>
    where
        S: 'static + StorageType,
    {
        let storage = storage_type.init()?;
        Ok(Mapping {
            key_layout,
            value_layout,
            storage_type: Arc::new(storage_type),
            storage: Some(storage),
            _pin: std::marker::PhantomPinned,
        })
    }

    pub(crate) fn read(&self, f: ZipFile<'_>) -> Result<Self, Error> {
        let storage = self.storage_type.read(f)?;
        Ok(Mapping {
            key_layout: self.key_layout.clone(),
            value_layout: self.value_layout.clone(),
            storage_type: self.storage_type.clone(),
            storage: Some(storage),
            _pin: std::marker::PhantomPinned,
        })
    }

    pub(crate) fn dump(&self) -> Result<Vec<u8>, Error> {
        self.storage
            .as_ref()
            .expect("storage not initialized")
            .dump()
    }

    pub fn is_initialized(&self) -> bool {
        self.storage.is_some()
    }

    pub fn key_layout(&self) -> &Layout {
        &self.key_layout
    }

    pub fn value_layout(&self) -> &Layout {
        &self.value_layout
    }

    pub(crate) fn insert(&mut self, key: Buffer, value: Buffer) {
        self.storage
            .as_mut()
            .expect("storage not initialized")
            .insert(hash(&key), value);
    }

    pub fn get(&self, key: Buffer) -> Option<&Buffer> {
        self.storage.as_ref().and_then(|s| s.get(hash(&key)))
    }

    unsafe fn call_mapping(mapping: *const Mapping, hash: u64) -> *const u8 {
        let mapping = &*mapping;
        if let Some(line) = mapping.storage.as_ref().and_then(|s| s.get(hash)) {
            line.as_ptr()
        } else {
            std::ptr::null()
        }
    }

    pub fn render(&self, func_name: String) -> qbe::Function<'static> {
        let input_slots = self.key_layout.slots();
        let args = input_slots
            .iter()
            .enumerate()
            .map(|(i, ty)| (ty.render(), qbe::Value::Temporary(format!("i{i}"))))
            .collect::<Vec<_>>();
        let mut func = qbe::Function::new(
            qbe::Linkage::private(),
            func_name,
            args,
            Some(qbe::Type::Long),
        );
        func.add_block("start");

        let hash = qbe::Value::Temporary("hash".to_string());

        func.assign_instr(
            hash.clone(),
            qbe::Type::Long,
            qbe::Instr::Copy(qbe::Value::Const(0)),
        );

        for (i, ty) in input_slots.iter().enumerate() {
            func.assign_instr(
                qbe::Value::Temporary(format!("cast_i{i}")),
                qbe::Type::Long,
                if ty.render() != qbe::Type::Long {
                    qbe::Instr::Cast(qbe::Value::Temporary(format!("i{i}")))
                } else {
                    qbe::Instr::Copy(qbe::Value::Temporary(format!("i{i}")))
                },
            );

            func.assign_instr(
                hash.clone(),
                qbe::Type::Long,
                qbe::Instr::Call(
                    qbe::Value::Const(update_hash as usize as u64),
                    vec![
                        (qbe::Type::Long, hash.clone()),
                        (qbe::Type::Long, qbe::Value::Temporary(format!("cast_i{i}"))),
                    ],
                ),
            );
        }

        let mapping_ptr = self as *const Mapping;
        func.assign_instr(
            qbe::Value::Temporary("slice".to_string()),
            qbe::Type::Long,
            qbe::Instr::Call(
                qbe::Value::Const(Mapping::call_mapping as usize as u64),
                vec![
                    (qbe::Type::Long, qbe::Value::Const(mapping_ptr as u64)),
                    (qbe::Type::Long, hash.clone()),
                ],
            ),
        );

        func.add_instr(qbe::Instr::Ret(Some(qbe::Value::Temporary(
            "slice".to_string(),
        ))));

        func
    }
}
