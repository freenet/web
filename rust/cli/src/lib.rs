pub mod crypto;
pub mod armorable;

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
