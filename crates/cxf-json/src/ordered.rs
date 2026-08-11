use std::{fmt, ops::Range, sync::Arc};

use crate::SourceDocument;

pub(crate) struct OrderedDocument {
    source: SourceDocument,
    root: OrderedValue,
}

impl fmt::Debug for OrderedDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrderedDocument")
            .field("source", &self.source)
            .field("root_kind", &self.root.kind())
            .finish_non_exhaustive()
    }
}

impl OrderedDocument {
    pub(crate) const fn new(source: SourceDocument, root: OrderedValue) -> Self {
        Self { source, root }
    }

    pub(crate) fn source_document(&self) -> &SourceDocument {
        &self.source
    }

    pub(crate) fn into_source_document(self) -> SourceDocument {
        self.source
    }

    #[cfg(test)]
    pub(crate) const fn root(&self) -> &OrderedValue {
        &self.root
    }
}

#[derive(Debug)]
pub(crate) enum OrderedValue {
    Null {
        token: Range<usize>,
    },
    Boolean {
        value: bool,
        token: Range<usize>,
    },
    Number {
        token: Range<usize>,
    },
    String {
        value: Arc<str>,
        token: Range<usize>,
    },
    Array {
        values: Vec<OrderedValue>,
        token: Range<usize>,
    },
    Object {
        members: Vec<OrderedMember>,
        token: Range<usize>,
    },
}

impl OrderedValue {
    const fn kind(&self) -> &'static str {
        match self {
            Self::Null { .. } => "null",
            Self::Boolean { .. } => "boolean",
            Self::Number { .. } => "number",
            Self::String { .. } => "string",
            Self::Array { .. } => "array",
            Self::Object { .. } => "object",
        }
    }

    pub(crate) fn token(&self) -> &Range<usize> {
        match self {
            Self::Null { token }
            | Self::Boolean { token, .. }
            | Self::Number { token }
            | Self::String { token, .. }
            | Self::Array { token, .. }
            | Self::Object { token, .. } => token,
        }
    }
}

impl Drop for OrderedValue {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut value) = pending.pop() {
            take_children(&mut value, &mut pending);
        }
    }
}

fn take_children(value: &mut OrderedValue, pending: &mut Vec<OrderedValue>) {
    match value {
        OrderedValue::Array { values, .. } => pending.append(values),
        OrderedValue::Object { members, .. } => pending.extend(
            std::mem::take(members)
                .into_iter()
                .map(|member| member.value),
        ),
        OrderedValue::Null { .. }
        | OrderedValue::Boolean { .. }
        | OrderedValue::Number { .. }
        | OrderedValue::String { .. } => {}
    }
}

#[derive(Debug)]
pub(crate) struct OrderedMember {
    pub(crate) name: Arc<str>,
    pub(crate) name_token: Range<usize>,
    pub(crate) value: OrderedValue,
}
