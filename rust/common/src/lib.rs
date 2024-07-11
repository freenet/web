use base64::{engine::general_purpose, Engine as _};

pub fn armor(data: &[u8], header: &str, footer: &str) -> String {
    let base64_data = general_purpose::STANDARD.encode(data);
    let wrapped_data = base64_data
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<&str>>()
        .join("\n");

    format!("-----BEGIN {}-----\n{}\n-----END {}-----", header, wrapped_data, footer)
}

pub fn dearmor(armored_text: &str, expected_header: &str, expected_footer: &str) -> Result<Vec<u8>, String> {
    let lines: Vec<&str> = armored_text.lines().collect();
    if lines.len() < 3 {
        return Err("Invalid armored text".to_string());
    }

    let header = lines[0];
    let footer = lines[lines.len() - 1];
    if header != format!("-----BEGIN {}-----", expected_header) || footer != format!("-----END {}-----", expected_footer) {
        return Err("Armored text does not match expected header/footer".to_string());
    }

    let base64_data: String = lines[1..lines.len() - 1].join("");
    general_purpose::STANDARD.decode(base64_data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_armor_dearmor() {
        let data = b"Hello, world!";
        let header = "TEST HEADER";
        let footer = "TEST FOOTER";

        let armored = armor(data, header, footer);
        let dearmored = dearmor(&armored, header, footer).unwrap();

        assert_eq!(data.to_vec(), dearmored);
    }
}
