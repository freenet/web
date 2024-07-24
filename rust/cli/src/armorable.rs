use ciborium::{de::from_reader, ser::into_writer};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::any::type_name;
use std::path::Path;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

pub trait Armorable: Serialize + for<'de> Deserialize<'de> {
    fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf)?;
        Ok(buf)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let object: Self = from_reader(bytes)?;
        Ok(object)
    }
    fn struct_name() -> String {
        let full_name = type_name::<Self>();
        let parts: Vec<&str> = full_name.split("::").collect();
        let struct_name = parts.last().unwrap_or(&full_name);
        Self::camel_case_to_upper(struct_name)
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

    fn to_file(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let buf = self.to_bytes()?;
        let base64_encoded = BASE64_STANDARD.encode(&buf);
        let wrapped = base64_encoded
            .as_bytes()
            .chunks(64)
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()?
            .join("\n");

        let struct_name = Self::struct_name();
        let pem_content = format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----\n",
            struct_name, wrapped, struct_name
        );

        let mut file = File::create(file_path)?;
        file.write_all(pem_content.as_bytes())?;
        Ok(())
    }

    fn from_file(file_path: &Path) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut pem_content = String::new();
        reader.read_to_string(&mut pem_content)?;

        let struct_name = Self::struct_name();
        let _begin_label = format!("-----BEGIN {}-----", struct_name);
        let _end_label = format!("-----END {}-----", struct_name);

        let base64_encoded = pem_content
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<Vec<&str>>()
            .join("");

        let decoded = BASE64_STANDARD.decode(&base64_encoded)?;
        Self::from_bytes(&decoded)
    }

    fn to_base64(&self) -> Result<String, Box<dyn std::error::Error>> {
        let buf = self.to_bytes()?;
        Ok(BASE64_STANDARD.encode(&buf))
    }

    fn from_base64(encoded: &str) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let decoded = BASE64_STANDARD.decode(encoded)?;
        Self::from_bytes(&decoded)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    impl Armorable for TestStruct {}

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
}
