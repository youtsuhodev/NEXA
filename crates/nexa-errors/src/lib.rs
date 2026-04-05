//! Centralized diagnostics for the NEXA compiler.

use std::fmt;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn point(pos: u32) -> Self {
        Self { start: pos, end: pos }
    }

    pub fn cover(a: Span, b: Span) -> Self {
        Self {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    pub fn spanned(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Diagnostic::new(message));
    }

    pub fn error_at(&mut self, span: Span, message: impl Into<String>) {
        self.push(Diagnostic::spanned(message, span));
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }
}

/// Format a diagnostic with optional source snippet (line/column).
pub fn format_diagnostic(path: &Path, source: &str, d: &Diagnostic) -> String {
    let mut out = String::new();
    if let Some(span) = d.span {
        if let Some((line, col, line_text)) = span_to_line_col(source, span) {
            out.push_str(&format!(
                "{}:{}:{}: error: {}\n",
                path.display(),
                line,
                col,
                d.message
            ));
            out.push_str(line_text);
            out.push('\n');
            let pad = col.saturating_sub(1);
            out.push_str(&" ".repeat(pad));
            let width = (span.end.saturating_sub(span.start)).max(1) as usize;
            out.push_str(&"^".repeat(width.min(line_text.len().saturating_sub(pad))));
            out.push('\n');
            return out;
        }
    }
    out.push_str(&format!("{}: error: {}\n", path.display(), d.message));
    out
}

fn span_to_line_col(source: &str, span: Span) -> Option<(usize, usize, &str)> {
    let start = span.start as usize;
    if start > source.len() {
        return None;
    }
    let before = &source[..start];
    let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[line_start..]
        .find('\n')
        .map(|i| line_start + i)
        .unwrap_or(source.len());
    let col = start - line_start + 1;
    Some((line, col, &source[line_start..line_end]))
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for d in &self.items {
            writeln!(f, "{}", d.message)?;
        }
        Ok(())
    }
}
