use std::collections::HashMap;
use std::io::{Read, Seek, Write};
use zip::write::SimpleFileOptions;

use crate::Error;

use super::{check, Graph};

impl Graph {
    // NOTE: need to use a concrete type because the `Storage` object that backs mappings
    // needs to be object-safe. A better workaround may be thought in the future.
    pub fn dump<W: Write + Seek>(&self, writer: W) -> Result<(), Error> {
        let mut writer = zip::ZipWriter::new(writer);

        writer.start_file("graph", SimpleFileOptions::default())?;
        bincode::serialize_into(&mut writer, self).map_err(Error::Bincode)?;

        // This is the authoritative value of metadata. Why? Because it's easy to load without
        // bloating the memory.
        writer.start_file("metadata.json", SimpleFileOptions::default())?;
        serde_json::to_writer(&mut writer, &self.metadata).map_err(Error::Json)?;

        for (name, mapping) in &self.mappings {
            writer.start_file(format!("{name}.mapping"), SimpleFileOptions::default())?;
            writer.write_all(&mapping.dump()?)?;
        }

        for (name, resources) in &self.resources {
            writer.start_file(format!("{name}.resource"), SimpleFileOptions::default())?;
            writer.write_all(&resources.dump()?)?;
        }

        writer.finish()?;

        Ok(())
    }

    /// Loads a graph in an unintialized state. This is quicker, since extra resources are
    /// not loader. However, you will not be able to compile the resultin graph.
    pub fn load_metadata<R: Read + Seek>(reader: R) -> Result<HashMap<String, String>, Error> {
        let mut archive = zip::ZipArchive::new(reader)?;
        let file = archive.by_name("metadata.json")?;
        let metadata: HashMap<String, String> =
            serde_json::from_reader(file).map_err(Error::Json)?;

        Ok(metadata)
    }

    /// Loads a graph in an unintialized state. This is quicker, since extra resources are
    /// not loader. However, you will not be able to compile the resulting graph.
    pub fn load_uninitialized<R: Read + Seek>(reader: R) -> Result<Self, Error> {
        let mut archive = zip::ZipArchive::new(reader)?;

        let file = archive.by_name("graph")?;
        let mut graph: Graph = bincode::deserialize_from(file).map_err(Error::Bincode)?;

        let file = archive.by_name("metadata.json")?;
        let metadata: HashMap<String, String> =
            serde_json::from_reader(file).map_err(Error::Json)?;
        graph.metadata = metadata;

        Ok(graph)
    }

    pub fn load<R: Read + Seek>(reader: R) -> Result<Self, Error> {
        let mut archive = zip::ZipArchive::new(reader)?;

        let file = archive.by_name("graph")?;
        let mut graph: Graph = bincode::deserialize_from(file).map_err(Error::Bincode)?;

        let file = archive.by_name("metadata.json")?;
        let metadata: HashMap<String, String> =
            serde_json::from_reader(file).map_err(Error::Json)?;
        graph.metadata = metadata;

        for id in 0..archive.len() {
            let file = archive.by_index(id)?;
            let Some(file_name) = file.name().strip_suffix(".mapping") else {
                continue;
            };
            let Some(mapping) = graph.mappings.get_mut(file_name) else {
                continue;
            };

            *mapping = mapping.read(file)?.into();
        }

        for id in 0..archive.len() {
            let file = archive.by_index(id)?;
            let Some(file_name) = file.name().strip_suffix(".resource") else {
                continue;
            };
            let Some(resource) = graph.resources.get_mut(file_name) else {
                continue;
            };

            *resource = resource.read(file)?.into();
        }

        check::run_checks(&mut graph)?;

        Ok(graph)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("can always serialize")
    }
}
