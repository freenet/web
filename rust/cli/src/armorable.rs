use serde::{Serialize, Deserialize};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::io::{self, BufReader, BufRead};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArmorableError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CBOR serialization error: {0}")]
    Cbor(#[from] serde_cbor::Error),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

// Define the Armorable trait
pub trait Armorable: Serialize + for<'de> Deserialize<'de> {
    // Static method to get the armor label for the type
    fn armor_label() -> &'static str;

    // Serialize the object to a base64 armored string
    fn to_base64_armored(&self) -> Result<String, ArmorableError> {
        let cbor_data = serde_cbor::to_vec(self)?;
        let base64_data = general_purpose::STANDARD.encode(&cbor_data);
        let wrapped_base64 = wrap_at_64_chars(&base64_data);
        Ok(format!(
            "-----BEGIN {}-----\n{}\n-----END {}-----",
            Self::armor_label(),
            wrapped_base64,
            Self::armor_label()
        ))
    }

    // Deserialize the object from a base64 armored string
    fn from_base64_armored(armored: &str) -> Result<Self, ArmorableError> where Self: Sized {
        let label = extract_label(armored)?;
        if label != Self::armor_label() {
            return Err(ArmorableError::Io(io::Error::new(io::ErrorKind::InvalidData, "Armor label does not match")));
        }
        let base64_data = extract_base64(armored, &label)?;
        let cbor_data = general_purpose::STANDARD.decode(&base64_data)?;
        let object = serde_cbor::from_slice(&cbor_data)?;
        Ok(object)
    }

    // Serialize the object to a naked base64 string
    fn to_naked_base64(&self) -> Result<String, ArmorableError> {
        let cbor_data = serde_cbor::to_vec(self)?;
        Ok(general_purpose::STANDARD.encode(&cbor_data))
    }

    // Deserialize the object from a naked base64 string
    fn from_naked_base64(base64: &str) -> Result<Self, ArmorableError> where Self: Sized {
        let cbor_data = general_purpose::STANDARD.decode(base64)?;
        let object = serde_cbor::from_slice(&cbor_data)?;
        Ok(object)
    }

    // Load the object from a file that may contain multiple armored blocks
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ArmorableError> where Self: Sized {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut in_block = false;
        let mut armored = String::new();
        let mut found_block = false;
        let label = Self::armor_label();

        for line in reader.lines() {
            let line = line?;
            if line.starts_with(&format!("-----BEGIN {}", label)) {
                if found_block {
                    return Err(ArmorableError::Io(io::Error::new(io::ErrorKind::InvalidData, "Multiple blocks with the same label found")));
                }
                in_block = true;
                found_block = true;
            } else if line.starts_with(&format!("-----END {}", label)) {
                in_block = false;
            } else if in_block {
                armored.push_str(&line);
                armored.push('\n');
            }
        }

        if !found_block {
            return Err(ArmorableError::Io(io::Error::new(io::ErrorKind::InvalidData, "No block with the specified label found")));
        }

        Self::from_base64_armored(&armored)
    }
}

// Helper function to wrap a string at 64 characters
fn wrap_at_64_chars(data: &str) -> String {
    data.as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<&str>>()
        .join("\n")
}

// Helper function to extract the armor label from an armored string
fn extract_label(armored: &str) -> io::Result<String> {
    if let Some(start) = armored.find("-----BEGIN ") {
        if let Some(end) = armored[start..].find("-----") {
            return Ok(armored[start + 11..start + end].trim().to_string());
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid armor label"))
}

// Helper function to extract the base64 data from an armored string
fn extract_base64(armored: &str, label: &str) -> io::Result<String> {
    let begin_label = format!("-----BEGIN {}-----", label);
    let end_label = format!("-----END {}-----", label);

    let start = armored.find(&begin_label)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "BEGIN label not found"))? + begin_label.len();
    let end = armored.find(&end_label)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "END label not found"))?;

    Ok(armored[start..end].replace("\n", ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    struct TestStruct {
        data: String,
    }

    impl Armorable for TestStruct {
        fn armor_label() -> &'static str {
            "TEST_STRUCT"
        }
    }

    #[test]
    fn test_armor_serde() {
        let test_obj = TestStruct { data: "Hello, world!".to_string() };

        // Test base64 armored serialization and deserialization
        let armored = test_obj.to_base64_armored().unwrap();
        let deserialized: TestStruct = TestStruct::from_base64_armored(&armored).unwrap();
        assert_eq!(test_obj.data, deserialized.data);

        // Test naked base64 serialization and deserialization
        let naked_base64 = test_obj.to_naked_base64().unwrap();
        let deserialized: TestStruct = TestStruct::from_naked_base64(&naked_base64).unwrap();
        assert_eq!(test_obj.data, deserialized.data);

        // Test loading from file
        let path = "test_file.txt";
        {
            let mut file = fs::File::create(path).unwrap();
            file.write_all(armored.as_bytes()).unwrap();
        }
        let deserialized_from_file: TestStruct = TestStruct::from_file(path).unwrap();
        assert_eq!(test_obj.data, deserialized_from_file.data);

        // Test error when multiple blocks are present
        {
            let mut file = fs::File::create(path).unwrap();
            file.write_all(armored.as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
            file.write_all(armored.as_bytes()).unwrap();
        }
        let result: io::Result<TestStruct> = TestStruct::from_file(path);
        assert!(result.is_err());
    }
}
