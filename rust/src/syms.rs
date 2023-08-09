use elf;

#[derive(Default, Clone)]
pub struct Symbol<'a> {
    name: &'a str,
    addr: u64,
    size: u64
}

pub type Symbols<'a> = Vec<Symbol<'a>>;

pub fn get_symbols<'a>(elf_file: &elf::ElfBytes<'a, elf::endian::AnyEndian>) -> Symbols<'a> {
    let (symtab, strtab) = match elf_file.symbol_table() {
        Ok(Some((symtab, strtab))) => (symtab, strtab),
        Ok(None) | Err(_) => return Symbols::default()
    };

    let mut symbols = Symbols::new();

    for sym in symtab {
        if let Ok(name) = strtab.get(sym.st_name as usize) {
            symbols.push(Symbol {
                name,
                addr: sym.st_value,
                size: sym.st_size
            });
        }
    }

    symbols.sort_by(|a, b| a.addr.cmp(&b.addr));
    symbols
}

/*
pub fn get_symbol<'a>(symbols: &Symbols<'a>, vaddr: u64) -> Option<(&'a str, u64)> {
    // TODO: Build a range tree? Or at least do a binary search?
    symbols
        .iter()
        .find(|sym| sym.addr <= vaddr && vaddr < sym.addr + sym.size)
        .map(|sym| (sym.name, sym.addr))
}
*/

#[derive(Debug, Clone)]
pub struct SymbolTreeNode<'a> {
    start: u64,
    size: u64,
    name: &'a str,
    left: Option<Box<SymbolTreeNode<'a>>>,
    right: Option<Box<SymbolTreeNode<'a>>>
}

impl<'a> SymbolTreeNode<'a> {
    pub fn build<'b>(slice: &[Symbol<'b>]) -> Option<Box<SymbolTreeNode<'b>>> {
        if slice.is_empty() {
            return None
        }

        let mid = slice.len() / 2;
        let symbol = &slice[mid];
        Some(Box::new(SymbolTreeNode {
            start: symbol.addr,
            size: symbol.size,
            name: symbol.name,
            left: Self::build(&slice[0..mid]),
            right: Self::build(&slice[mid+1..slice.len()])
        }))
    }

    pub fn lookup(&self, value: u64) -> Option<(&'a str, u64)> {
        if value < self.start {
            return self.left.as_ref().map(|node| node.lookup(value)).flatten()
        }

        if value < self.start + self.size {
            return Some((self.name, self.start))
        }

        assert!(self.start + self.size <= value);
        return self.right.as_ref().map(|node| node.lookup(value)).flatten()
    }

    pub fn count(&self) -> usize {
        1 +
            self.left.as_ref().map(|node| node.count()).unwrap_or(0) +
            self.right.as_ref().map(|node| node.count()).unwrap_or(0)
    }
}



