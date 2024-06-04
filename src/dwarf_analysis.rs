use std::{borrow, collections::HashMap, fs};

use object::{Object, ObjectSection};

use crate::error;
use error::Result;

/// Parse an ELF file to determine the programming language used.
///
/// This function analyzes the Dwarf information in the ELF file to determine the programming language used.
///
/// # Arguments
///
/// * `file_path` - The path to the ELF file.
///
/// # Returns
///
/// Returns a `Result` containing the programming language used, if successfully determined.
/// Analysis example from: <https://github.com/gimli-rs/gimli/blob/master/crates/examples/src/bin/simple.rs>
pub fn dwarf_analysis(file_path: &str) -> Result<String> {
    let file = fs::File::open(file_path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let object = object::File::parse(&*mmap)?;
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    let lang = analyze_elf_file(&object, endian)?;
    Ok(lang)
}

// Parse the dwarf format in the .debug_info section. Language attributes table available here: https://dwarfstd.org/languages.html
fn analyze_elf_file<'b>(
    object: &'b object::File<'b>,
    endian: gimli::RunTimeEndian,
) -> Result<String> {
    let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>> {
        match object.section_by_name(id.name()) {
            Some(ref section) => Ok(section
                .uncompressed_data()
                .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
            None => Ok(borrow::Cow::Borrowed(&[][..])),
        }
    };

    let mut language_counts = HashMap::new();
    let dwarf_cow = gimli::Dwarf::load(&load_section)?;
    let borrow_section: &dyn for<'a> Fn(
        &'a borrow::Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(section, endian);

    let dwarf = dwarf_cow.borrow(&borrow_section);
    let mut iter = dwarf.units();

    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();

        while let Some((_, entry)) = entries.next_dfs()? {
            if let Some(language_attr) = entry.attr_value(gimli::DW_AT_language)? {
                let language = match language_attr {
                    gimli::AttributeValue::Language(language) => language,
                    _ => continue,
                };
                increment_language_count(&mut language_counts, &language.to_string());
            }
        }
    }
    let mut max_count = 0;
    let mut max_language = "".to_string();

    // The presence of C99 in the Rust program is due to the musl library, used to statically compile the binary
    if language_counts.contains_key("DW_LANG_C99") && language_counts.contains_key("DW_LANG_Rust") {
        language_counts.remove_entry("DW_LANG_C99");
    }
    for (language, count) in language_counts {
        if count > max_count {
            max_count = count;
            max_language = language.clone();
        }
    }
    Ok(max_language)
}

fn increment_language_count(map: &mut HashMap<String, u32>, language: &str) {
    let count = map.entry(language.to_string()).or_insert(0);
    *count += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dwarf_analysis() {
        let file_path = "./tests/elf_file/fake-firmware-rust-dynamic";
        let result = dwarf_analysis(file_path).unwrap();
        assert_eq!(result, "DW_LANG_Rust".to_string());
    }

    #[test]
    fn test_analyze_elf_file() {
        let file = fs::File::open("./tests/elf_file/fake-firmware-rust-dynamic").unwrap();
        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
        let object = object::File::parse(&*mmap).unwrap();
        let endian = gimli::RunTimeEndian::Little;
        let result = analyze_elf_file(&object, endian).unwrap();
        assert_eq!(result, "DW_LANG_Rust");
    }
}
