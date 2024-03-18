#[derive(Default, Clone)]
pub struct Symbol<'a> {
    name: &'a str,
    addr: i64,
    size: i64
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
                addr: sym.st_value as i64,
                size: sym.st_size as i64
            });
        }
    }

    symbols.sort_by(|a, b| a.addr.cmp(&b.addr));
    symbols
}

#[derive(Debug, Clone)]
pub struct SymbolTreeNode<'a> {
    start: i64,
    size: i64,
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

    pub fn lookup(&self, value: i64) -> Option<(&'a str, i64)> {
        if value < self.start {
            return self.left.as_ref().and_then(|node| node.lookup(value))
        }

        if value < self.start + self.size {
            return Some((self.name, self.start))
        }

        assert!(self.start + self.size <= value);
        self.right.as_ref().and_then(|node| node.lookup(value))
    }

    #[allow(unused)]
    pub fn count(&self) -> usize {
        1 +
            self.left.as_ref().map(|node| node.count()).unwrap_or(0) +
            self.right.as_ref().map(|node| node.count()).unwrap_or(0)
    }
}



