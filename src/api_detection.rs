use goblin::elf::Elf;

use crate::{cleanup::demangle_func_name, elf_utils, error};
use elf_utils::API;
use error::Result;

/// Do an API lookup in the symbol table.
///
/// This function searches for APIs in the symbol table of the ELF file based on a list of API names provided.
///
/// # Arguments
///
/// * `elf` - The ELF file structure.
/// * `api_list` - A vector containing the names of the APIs to search for.
///
/// # Returns
///
/// Returns a `Result` containing a vector of `API` structures representing the APIs found.
pub fn func_search<'a>(elf: &'a Elf<'a>, language: &str) -> Result<Vec<API>> {
    let mut func_found = Vec::new();
    for symbol in &elf.syms {
        if symbol.st_type() == goblin::elf::sym::STT_FUNC && symbol.st_shndx != 0 {
            if let Some(function_name) = get_name_sym(elf, &symbol.to_owned()) {
                let demangled_name = demangle_func_name(function_name, language)?;
                func_found.push(API::new(
                    demangled_name,
                    symbol.st_value,
                    symbol.st_value + symbol.st_size,
                ));
            }
        }
    }
    Ok(func_found)
}

// This function retrieves the name of a symbol from the ELF symbol table.
fn get_name_sym<'a>(elf: &'a Elf, symbol: &'a goblin::elf::Sym) -> Option<&'a str> {
    let name_offset = symbol.st_name;
    let name_str: &'a str = elf.strtab.get_at(name_offset)?;
    Some(name_str)
}

pub fn extract_api(name: &str, func_found: Vec<API>) -> Option<API>{
    for api in func_found.iter(){
        if api.name.contains(name) {
            return Some(api.clone());
        }
    }
    None
}
