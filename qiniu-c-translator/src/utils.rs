#![allow(dead_code)]

use std::{
    fs::File,
    io::{Result, Stdout, Write},
    iter,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

pub trait CodeGenerator {
    fn generate_code(&self, output: CodeWriter) -> Result<CodeWriter>;
}

#[derive(Debug)]
pub enum Writer {
    Stdout(Stdout),
    Memory(Vec<u8>),
    File(File),
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::Stdout(out) => out.write(buf),
            Self::Memory(mem) => mem.write(buf),
            Self::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Stdout(out) => out.flush(),
            Self::Memory(mem) => mem.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

pub struct CodeWriter {
    writer: Writer,
    tab_spaces: usize,
    level: usize,
}

impl CodeWriter {
    pub fn new(writer: Writer, tab_spaces: usize, level: usize) -> Self {
        Self {
            writer,
            tab_spaces,
            level,
        }
    }

    pub fn try_with_next_level<F: FnOnce(Self) -> Result<Self>>(mut self, f: F) -> Result<Self> {
        self.level += 1;
        self = f(self)?;
        self.level -= 1;
        Ok(self)
    }

    pub fn write(&mut self, code: &str) -> Result<()> {
        code.lines().try_for_each(|line| {
            self.writer
                .write_fmt(format_args!("{}{}\n", self.generate_prefix_spaces(), line))
        })
    }

    fn generate_prefix_spaces(&self) -> String {
        iter::repeat(' ').take(self.level * self.tab_spaces).collect()
    }

    #[inline]
    pub fn into_inner(self) -> Writer {
        self.writer
    }
}

pub struct RandomIdentifier {
    id: AtomicUsize,
}

impl Default for RandomIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomIdentifier {
    pub fn new() -> Self {
        Self {
            id: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn upper_camel_case(&self) -> String {
        self.generate("Internal__identify__")
    }

    #[inline]
    pub fn pascal_case(&self) -> String {
        self.upper_camel_case()
    }

    #[inline]
    pub fn lower_camel_case(&self) -> String {
        self.generate("__internal__identify__")
    }

    #[inline]
    pub fn dromedary_case(&self) -> String {
        self.lower_camel_case()
    }

    #[inline]
    pub fn snack_case(&self) -> String {
        self.generate("internal__identify_")
    }

    fn generate(&self, prefix: &str) -> String {
        format!("{}{}", prefix, self.id.fetch_add(1, Relaxed))
    }
}
