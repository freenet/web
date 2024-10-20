use std::any::type_name;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ciborium::{de::from_reader, ser::into_writer};
use serde::{Deserialize, Serialize};

use super::errors::GhostkeyError;
use super::errors::GhostkeyError::Base64DecodeError;

pub trait Armorable: Serialize + for<'de> Deserialize<'de> + 'static {
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

        let upper_name = Self::camel_case_to_upper(name);
        
        // Check for an existing version suffix
        if let Some(version_index) = upper_name.rfind('_') {
            let (_, version) = upper_name.split_at(version_index);
            if version.starts_with("_V") && version[2..].chars().all(|c| c.is_digit(10)) {
                return upper_name;
            }
        }

        // Add V1 if no version is present
        format!("{}_V1", upper_name)
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

    fn to_armored_string(&self) -> Result<String, GhostkeyError> {
        let buf = self
            .to_bytes()
            .map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        let base64_encoded = BASE64_STANDARD.encode(&buf);
        let wrapped = base64_encoded
            .as_bytes()
            .chunks(64)
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .map_err(|e| GhostkeyError::DecodingError(format!("UTF decoding error: {}", e)))?
            .join("\n");

        let struct_name = Self::struct_name();
        let pem_content = format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----\n",
            struct_name, wrapped, struct_name
        );

        Ok(pem_content)
    }

    fn to_file(&self, file_path: &Path) -> Result<(), GhostkeyError> {
        let pem_content = self.to_armored_string()?;
        let mut file = File::create(file_path).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        file.write_all(pem_content.as_bytes())
            .map_err(|e| GhostkeyError::IOError(e.to_string()))?;
        Ok(())
    }

    fn from_armored_string(armored_string: &str) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let struct_name = Self::struct_name();
        let possible_labels = vec![
            struct_name.clone(),
            struct_name.trim_end_matches("_V1").to_string(),
        ];

        for label in possible_labels {
            let begin_label = format!("-----BEGIN {}-----", label);
            let end_label = format!("-----END {}-----", label);

            if let Some(block) = armored_string.split(&begin_label).nth(1) {
                if let Some(content) = block.split(&end_label).next() {
                    let trimmed_content = content.trim();
                    match Self::decode_block(trimmed_content) {
                        Ok(result) => return Ok(result),
                        Err(_) => continue, // Try the next label if decoding fails
                    }
                }
            }
        }

        Err(GhostkeyError::DecodingError(format!(
            "Failed to decode any matching block for {}",
            struct_name
        )))
    }

    fn decode_block(block: &str) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let base64_encoded = block
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<Vec<&str>>()
            .join("");

        let decoded = BASE64_STANDARD
            .decode(&base64_encoded)
            .map_err(|e| GhostkeyError::Base64DecodeError(e.to_string()))?;
        Self::from_bytes(&decoded)
    }

    fn from_file(file_path: &Path) -> Result<Self, GhostkeyError>
    where
        Self: Sized,
    {
        let mut file = File::open(file_path).map_err(|e| GhostkeyError::IOError(e.to_string()))?;
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
        let decoded = BASE64_STANDARD
            .decode(encoded)
            .map_err(|e| Base64DecodeError(e.to_string()))?;
        Self::from_bytes(&decoded)
    }
}

impl<T: Serialize + for<'de> Deserialize<'de> + 'static> Armorable for T {}

#[cfg(test)]
mod tests {
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
    fn test_struct_name() {
        assert_eq!(TestStruct::struct_name(), "TEST_STRUCT_V1");
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SerializableTestStruct {
        field: String,
    }

    #[test]
    fn test_serializable_struct_name() {
        assert_eq!(SerializableTestStruct::struct_name(), "TEST_STRUCT_V1");
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

        let armored = test_struct1.to_armored_string().unwrap();
        let decoded_struct1 = TestStruct::from_armored_string(&armored).unwrap();

        assert_eq!(test_struct1, decoded_struct1);
    }

    #[test]
    fn test_mismatched_label_from_armored_string() {
        let test_struct1 = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let _armored = test_struct1.to_armored_string().unwrap();
        let mismatched_label = "-----BEGIN TEST_STRUCT-----\n\n-----END TEST_STRUCTS-----\n";
        let decoded_struct1 = TestStruct::from_armored_string(&mismatched_label);
        assert!(decoded_struct1.is_err());
    }

    // New tests for versioning scenarios

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStructV2 {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_struct_name_with_version() {
        assert_eq!(TestStructV2::struct_name(), "TEST_STRUCT_V2");
    }

    #[test]
    fn test_struct_no_version_implicit_v1() {
        let test_struct1 = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let armored = test_struct1.to_armored_string().unwrap();
        assert!(armored.contains("TEST_STRUCT_V1"));
    }

    #[test]
    fn test_struct_with_version_label_implicit_v1() {
        let test_struct_v1 = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let armored = test_struct_v1.to_armored_string().unwrap();
        let armored_no_version = armored.replace("TEST_STRUCT_V1", "TEST_STRUCT");

        let decoded_struct_v1 = TestStruct::from_armored_string(&armored_no_version).unwrap();
        assert_eq!(test_struct_v1, decoded_struct_v1);
    }

    #[test]
    fn test_struct_v1_label_with_version() {
        let test_struct_v1 = TestStruct {
            field1: "Hello".to_string(),
            field2: 42,
        };

        let armored = test_struct_v1.to_armored_string().unwrap();

        let decoded_struct_v1 = TestStruct::from_armored_string(&armored).unwrap();
        assert_eq!(test_struct_v1, decoded_struct_v1);
    }
}
