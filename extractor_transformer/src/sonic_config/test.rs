use alloy::transports::{RpcError, TransportErrorKind};
use log::info;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::{
    build_catalog, extract_transform,
    extraction::{DebugTraces, EthExtractor, EvmDebugExtractor, EvmExtracted},
    proto_codegen::etl::request::IndexingRequest,
    ExtractTransformErr,
};
use crate::blockchain_config::{build_provider, save_range, PerBlockRecords};

const EXT_FLDR_NAME: &str = "extraction";
const TRANS_FLDR_NAME: &str = "transformation";

pub struct TestSet {
    pub name: String,
    pub request: IndexingRequest,
    pub dirpath: PathBuf,
}

impl TestSet {
    pub fn create_new(
        test_dir: PathBuf,
        name: String,
        request: IndexingRequest,
    ) -> Result<Self, std::io::Error> {
        // Make sure all names are lowercase and alphabetic
        if !name.chars().all(|c| c.is_lowercase() && c.is_alphabetic()) {
            panic!("The name of the testset must only be lowercase alpha")
        }

        // Validate the top level directory exists
        if !test_dir.is_dir() {
            panic!("The filepath `{test_dir:#?}` does not point to an accessible directory");
        }

        // Get the directory path for this specific test set, verify doesn't exist
        let dirpath = test_dir.join(format!("{}_{}_{}", name, request.start, request.end));
        if dirpath.exists() {
            panic!("The filepath `{dirpath:#?}` already exists, erase it to build a new testset");
        }

        // Create new directory(s)
        match std::fs::create_dir(&dirpath) {
            Ok(_) => info!("Created a directory {dirpath:#?}"),
            Err(err) => return Err(err),
        }

        match std::fs::create_dir(dirpath.join(EXT_FLDR_NAME)) {
            Ok(_) => info!("Created directory for extracted data {dirpath:#?}"),
            Err(err) => return Err(err),
        }

        match std::fs::create_dir(dirpath.join(TRANS_FLDR_NAME)) {
            Ok(_) => info!("Created directory for transformed data {dirpath:#?}"),
            Err(err) => return Err(err),
        }

        Ok(Self {
            name,
            request,
            dirpath,
        })
    }

    pub fn load_existing(path: &PathBuf) -> Self {
        if !path.exists() {
            panic!("Path {path:?} does not exist, thus cannot be a testset");
        } else if !path.is_dir() {
            panic!("Path {path:?} is not a directory, thus cannot be a testset");
        }

        let (name, request) = Self::name_and_request_from_path(path);

        Self {
            name: name.to_string(),
            request,
            dirpath: path.clone(),
        }
    }

    pub fn name_and_request_from_path(path: &Path) -> (&str, IndexingRequest) {
        let (name, start, end) = match path.file_name() {
            Some(dirname) => match dirname.to_str() {
                Some(string) => {
                    let split: Vec<_> = string.split('_').collect();
                    let name = split[0];
                    let start = split[1].parse().expect("Invalid number");
                    let end = split[2].parse().expect("Invalid number");
                    (name, start, end)
                }
                None => panic!("Invalid name"),
            },
            None => panic!("Unable to get file_name"),
        };

        (
            name,
            IndexingRequest {
                start,
                end,
                ..Default::default()
            },
        )
    }

    #[inline]
    pub fn name_and_request(&self) -> (&str, IndexingRequest) {
        Self::name_and_request_from_path(&self.dirpath)
    }

    pub fn extraction_path(&self) -> PathBuf {
        self.dirpath.join(EXT_FLDR_NAME)
    }

    pub fn transformation_path(&self) -> PathBuf {
        self.dirpath.join(TRANS_FLDR_NAME)
    }

    pub async fn save_extractions(&self) -> Result<(), Option<RpcError<TransportErrorKind>>> {
        save_range(self.request.clone(), None, None, &self.extraction_path()).await
    }

    pub async fn save_transformations(&self) -> Result<(), Vec<(u64, ExtractTransformErr)>> {
        let provider = build_provider();

        for block_number in self.request.start..self.request.end {
            let perblock = extract_transform(
                block_number,
                None,
                Some(self.request.clone()),
                Some(provider.clone()),
                build_catalog(),
            )
            .await
            .expect("Failed to extract_transform");

            let filepath = self
                .transformation_path()
                .join(format!("{}.json", block_number));

            let file = File::create(filepath).expect("Failed to create a file");

            serde_json::to_writer(file, &perblock).expect("Failed to write to file");
        }

        Ok(())
    }

    pub async fn validate_extraction(&self) {
        let extractor = EthExtractor::new(build_provider());

        for number in self.request.start..=self.request.end {
            let (evm, debug) = deserialize_extracted_files(&self.extraction_path(), number)
                .expect("Missing extractable files");

            let evm_now = extractor
                .extract_basic(number, Some(self.request.clone()), 5, 5)
                .await
                .expect("Failed")
                .expect("Bad index, returned None");
            assert_eq!(evm, evm_now, "Evm extractions don't match");
            let dbg_now = extractor
                .extract_debug(number)
                .await
                .expect("Failed")
                .expect("Bad index, returned None");
            assert_eq!(debug, dbg_now, "Debug extractions don't match");
        }
    }

    pub async fn validate_transformation(&self) {
        let (_, request) = self.name_and_request();
        let provider = build_provider();

        for entry in fs::read_dir(self.transformation_path()).expect("Failed to read directory") {
            let entry = entry.expect("Failed to get entry");
            let file_path = entry.path();

            if let Some(ext) = file_path.extension() {
                if ext != "json" {
                    continue;
                }
            } else {
                continue;
            }

            let file_stem = file_path
                .file_name()
                .expect("Failed to get file_name from file_path")
                .to_str()
                .and_then(|name| name.strip_suffix(".json"));

            let block_number = match file_stem.map(|stem| stem.parse::<u64>()) {
                Some(Ok(block_number)) => block_number,
                Some(Err(_)) => continue,
                None => continue,
            };

            let file = File::open(&file_path).expect("Failed to open file");
            let deserialized_records: PerBlockRecords =
                serde_json::from_reader(file).expect("Failed to deserialize old records");

            let perblock = extract_transform(
                block_number,
                None,
                Some(request.clone()),
                Some(provider.clone()),
                build_catalog(),
            )
            .await
            .expect("Failed to extract_transform");

            assert_eq!(
                deserialized_records, perblock,
                "Saved records do not match extracted_transformed records"
            );
        }
    }
}

fn deserialize_extracted_files(
    extraction_path: &Path,
    number: u64,
) -> Result<(EvmExtracted, DebugTraces), Box<dyn std::error::Error>> {
    // Construct file paths
    let basic_path = extraction_path.join(format!("basic_{}.json", number));
    let debug_path = extraction_path.join(format!("debug_{}.json", number));

    // Read JSON files
    let basic_content = fs::read_to_string(&basic_path)?;
    let debug_content = fs::read_to_string(&debug_path)?;

    // Deserialize JSON to structs
    let evm_extracted: EvmExtracted = serde_json::from_str(&basic_content)?;
    let debug_traces: DebugTraces = serde_json::from_str(&debug_content)?;

    Ok((evm_extracted, debug_traces))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_all_test_cases() {
        // Loads the .env file, raises an error otherwise
        dotenvy::dotenv().expect(".env file is required");

        let test_dir = std::path::Path::new("tests");

        // Iterate through all subdirectories in the `tests` directory
        for entry in std::fs::read_dir(test_dir).expect("Failed to read test directory") {
            let entry = entry.expect("Invalid directory entry");
            let path = entry.path();

            if path.is_dir() {
                // Use the corrected `load_existing` to load the test set
                println!("Loading test case from directory: {:?}", path);

                let test_set = TestSet::load_existing(&path);

                // Validate extractions and transformations
                test_set.validate_extraction().await;
                test_set.validate_transformation().await;
            }
        }
    }
}
