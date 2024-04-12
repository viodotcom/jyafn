use std::io::{Read, Seek, Write};

use crate::Error;

use super::Graph;

impl Graph {
    // NOTE: need to use a concrete type because the `Storage` object that backs mappings
    // needs to be object-safe. A better workaround may be thought in the future.
    pub fn dump<W: Write + Seek>(&self, writer: W) -> Result<(), Error> {
        let mut writer = zip::ZipWriter::new(writer);

        writer.start_file("graph", Default::default())?;
        bincode::serialize_into(&mut writer, self).map_err(Error::Deserialization)?;

        for (name, mapping) in self.mappings() {
            writer.start_file(format!("{name}.mapping"), Default::default())?;
            writer.write_all(&mapping.dump()?)?;
        }

        writer.finish()?;

        Ok(())
    }

    /// Loads a graph in an unintialized state. This is quicker, since extra resources are
    /// not loader. However, you will not be able to compile the resultin graph.
    pub fn load_uninitialized<R: Read + Seek>(reader: R) -> Result<Self, Error> {
        let mut archive = zip::ZipArchive::new(reader)?;
        let file = archive.by_name("graph")?;

        bincode::deserialize_from(file).map_err(Error::Deserialization)
    }

    pub fn load<R: Read + Seek>(reader: R) -> Result<Self, Error> {
        let mut archive = zip::ZipArchive::new(reader)?;

        let file = archive.by_name("graph")?;
        let mut graph: Graph = bincode::deserialize_from(file).map_err(Error::Deserialization)?;

        for id in 0..archive.len() {
            let file = archive.by_index(id)?;
            let Some(file_name) = file.name().strip_suffix(".mapping") else {
                continue;
            };
            let Some(mapping) = graph.mappings_mut().get_mut(file_name) else {
                continue;
            };

            *mapping = mapping.read(file)?.into();
        }

        for (name, mapping) in graph.mappings() {
            if !mapping.is_initialized() {
                return Err(format!(
                    "while reading zip archive, mapping {name} was not initialized"
                )
                .into());
            }
        }

        Ok(graph)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("can always serialize")
    }
}
