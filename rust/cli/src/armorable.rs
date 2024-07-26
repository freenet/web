use ciborium::{de::from_reader, ser::into_writer};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::any::type_name;
use std::path::Path;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use crate::errors::GhostkeyError;
use crate::errors::GhostkeyError::Base64DecodeError;

pub trait Armorable: Serialize + for<'de> Deserialize<'de> {
    fn to_bytes(&self) -> Result<Vec<u8>, GhostkeyError> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        Ok(buf)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let object: Self = from_reader(bytes).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        Ok(object)
    }
    fn struct_name() -> String {
        let full_name = type_name::<Self>();
        let parts: Vec<&str> = full_name.split("::").collect();
        let struct_name = parts.last().unwrap_or(&full_name);
        let name = if struct_name.starts_with("Serializable") {
            &struct_name["Serializable".len()..]
        } else {
            struct_name
        };
        Self::camel_case_to_upper(name)
    }

    fn camel_case_to_upper(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i != 0 {
                result.push('_');
            }
            result.push(c);
        }
        result.to_uppercase()
    }

    fn to_file(&self, file_path: &Path) -> Result<(), GhostkeyError> {
        let buf = self.to_bytes().map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        let base64_encoded = BASE64_STANDARD.encode(&buf);
        let wrapped = base64_encoded
            .as_bytes()
            .chunks(64)
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>().map_err(|e| GhostkeyError::DecodingError(format!("UTF decoding error: {}", e)))?
            .join("\n");

        let struct_name = Self::struct_name();
        let pem_content = format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----\n",
            struct_name, wrapped, struct_name
        );

        let mut file = File::create(file_path).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        file.write_all(pem_content.as_bytes()).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        Ok(())
    }

    fn from_armored_string(armored_string: &str) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let struct_name = Self::struct_name();
        let begin_label = format!("-----BEGIN {}-----", struct_name);
        let end_label = format!("-----END {}-----", struct_name);

        let blocks: Vec<&str> = armored_string.split(&begin_label).collect();
        let matching_blocks: Vec<&str> = blocks.into_iter()
            .filter(|block| block.contains(&end_label))
            .collect();

        if matching_blocks.is_empty() {
            return Err(GhostkeyError::DecodingError(format!("No matching block found for {}", struct_name)));
        }

        for block in matching_blocks {
            if let Ok(result) = Self::decode_block(block, &end_label) {
                return Ok(result);
            }
        }

        Err(GhostkeyError::DecodingError(format!("Failed to decode any matching block for {}", struct_name)))
    }

    fn decode_block(block: &str, end_label: &str) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let base64_encoded = block
            .lines()
            .take_while(|line| !line.contains(end_label))
            .filter(|line| !line.starts_with("-----"))
            .collect::<Vec<&str>>()
            .join("");

        let decoded = BASE64_STANDARD.decode(&base64_encoded)
            .map_err(|e| GhostkeyError::Base64DecodeError(e.to_string()))?;
        Self::from_bytes(&decoded)
    }

    fn from_file(file_path: &Path) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let mut file = File::open(file_path)
            .map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        let mut armored_content = String::new();
        file.read_to_string(&mut armored_content)
            .map_err(|e| GhostkeyError::IOError(e.to_string()))?;

        Self::from_armored_string(&armored_content)
    }

    fn to_base64(&self) -> Result<String, Box<dyn std::error::Error>> {
        let buf = self.to_bytes()?;
        Ok(BASE64_STANDARD.encode(&buf))
    }

    fn from_base64(encoded: &str) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let decoded = BASE64_STANDARD.decode(encoded).map_err(|e| Base64DecodeError(e.to_string()))?;
        Self::from_bytes(&decoded)
    }
}

impl<T: Serialize + for<'de> Deserialize<'de>> Armorable for T {}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_to_base64() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let base64_result = test_struct.to_base64().unwrap();
        assert!(!base64_result.is_empty());
    }

    #[test]
    fn test_from_base64() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let base64_string = test_struct.to_base64().unwrap();
        let decoded_struct = TestStruct::from_base64(&base64_string).unwrap();

        assert_eq!(test_struct, decoded_struct);
    }

    #[test]
    fn test_to_file_and_from_file() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_struct.armored");

        test_struct.to_file(&file_path).unwrap();

        // dump file contents to stdout
        std::process::Command::new("cat")
            .arg(&file_path)
            .status()
            .expect("failed to execute process");
        
        let loaded_struct = TestStruct::from_file(&file_path).unwrap();

        assert_eq!(test_struct, loaded_struct);
    }

    #[test]
    fn test_struct_name() {
        assert_eq!(TestStruct::struct_name(), "TEST_STRUCT");
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SerializableTestStruct {
        field: String,
    }

    #[test]
    fn test_serializable_struct_name() {
        assert_eq!(SerializableTestStruct::struct_name(), "TEST_STRUCT");
    }

    #[test]
    fn test_camel_case_to_upper() {
        assert_eq!(
            TestStruct::camel_case_to_upper("camelCaseString"),
            "CAMEL_CASE_STRING"
        );
    }

    #[test]
    fn test_to_bytes_and_from_bytes() {
        let test_struct = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let bytes = test_struct.to_bytes().unwrap();
        let decoded_struct = TestStruct::from_bytes(&bytes).unwrap();

        assert_eq!(test_struct, decoded_struct);
    }

    #[test]
    fn test_from_armored_string() {
        let test_struct1 = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };
        let test_struct2 = TestStruct {
            field1: "World".to_string(),
            field2: 24,
        };

        let temp_dir = tempdir().unwrap();
        let file_path1 = temp_dir.path().join("test_struct1.armored");
        let file_path2 = temp_dir.path().join("test_struct2.armored");

        test_struct1.to_file(&file_path1).unwrap();
        test_struct2.to_file(&file_path2).unwrap();

        let armored_content1 = std::fs::read_to_string(&file_path1).unwrap();
        let armored_content2 = std::fs::read_to_string(&file_path2).unwrap();

        // Test single block
        let loaded_struct1 = TestStruct::from_armored_string(&armored_content1).unwrap();
        assert_eq!(test_struct1, loaded_struct1);

        // Test multiple blocks
        let combined_content = format!("{}\n{}", armored_content1, armored_content2);
        let loaded_struct2 = TestStruct::from_armored_string(&combined_content).unwrap();
        assert_eq!(test_struct1, loaded_struct2);

        // Test no matching block
        let wrong_content = "-----BEGIN WRONG_STRUCT-----\nwrongcontent\n-----END WRONG_STRUCT-----\n";
        assert!(TestStruct::from_armored_string(wrong_content).is_err());

        // Test multiple matching blocks (should now succeed)
        let duplicate_content = format!("{}\n{}", armored_content1, armored_content1);
        let loaded_struct3 = TestStruct::from_armored_string(&duplicate_content).unwrap();
        assert_eq!(test_struct1, loaded_struct3);
    }
}
