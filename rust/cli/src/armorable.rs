use ciborium::{de::from_reader, ser::into_writer};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::any::type_name;
use std::path::Path;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

pub trait Armorable: Serialize + for<'de> Deserialize<'de> {
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
                result.push(' ');
            }
            result.push(c);
        }
        result.to_uppercase()
    }

    fn to_file(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf)?;
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
        let begin_label = format!("-----BEGIN {}-----", struct_name);
        let end_label = format!("-----END {}-----", struct_name);

        let base64_encoded = pem_content
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<Vec<&str>>()
            .join("");

        let decoded = BASE64_STANDARD.decode(&base64_encoded)?;
        let object: Self = from_reader(&decoded[..])?;
        Ok(object)
    }

    fn to_base64(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf)?;
        let base64_encoded = BASE64_STANDARD.encode(&buf);
        Ok(base64_encoded)
    }

    fn from_base64(encoded: &str) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let decoded = BASE64_STANDARD.decode(encoded)?;
        let object: Self = from_reader(&decoded[..])?;
        Ok(object)
    }
}

// Blanket implementation for all types that implement Serialize and Deserialize
impl<T> Armorable for T where T: Serialize + for<'de> Deserialize<'de> {}

#[derive(Serialize, Deserialize, Debug)]
struct FooBar {
    foo: String,
    bar: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let foobar = FooBar {
        foo: "example".to_string(),
        bar: 42,
    };

    // Write to PEM file
    foobar.to_file(Path::new("foobar.pem"))?;
    // Read from PEM file
    let foobar_from_pem = FooBar::from_file(Path::new("foobar.pem"))?;
    println!("{:?}", foobar_from_pem);

    // Write to base64 string
    let base64_str = foobar.to_base64()?;
    println!("{}", base64_str);

    // Read from base64 string
    let foobar_from_base64 = FooBar::from_base64(&base64_str)?;
    println!("{:?}", foobar_from_base64);

    Ok(())
}
