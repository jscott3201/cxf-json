//! Crate-private W-013-C1 typed CXF projection over the ordered source view.
//!
//! The projected record classifies the OBC section 8.2 core vocabulary into
//! owned structures, keeps authored order, preserves unknown and weakly typed
//! terms as CXF extension records, and records private diagnostics for known
//! emitter damage. Per D-020, ordering comes from the retained source view,
//! never from RDF. Per D-030, this module performs no source-to-RDF identity
//! joins and no JSON-LD context expansion: compacted vocabulary spellings are
//! recognized only when the document's own `@context` maps a registered
//! prefix to its registered namespace IRI, and distinct IRIs keep distinct
//! internal identity (register rows C-001, C-002, C-016). Embedded
//! reference-object members beyond their first string `@id` and unhandled
//! keyword shapes are retained verbatim as extension records rather than
//! silently dropped. `@included` members collect nodes like `@graph`;
//! node-scoped `@context` members leave evidence but are never applied —
//! compacted registration follows the root context only. All behavior here
//! remains crate-private; profile 0.1.4 public exports are unchanged.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use crate::ordered::{OrderedDocument, OrderedMember, OrderedValue};

/// Namespace registration for one CXF vocabulary generation.
///
/// Namespace IRIs are internal identity. The legacy HTTPS S231P variant
/// (register row C-002) is intentionally a separate identity from the
/// canonical HTTP S231P namespace: the projection never merges them.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Vocabulary {
    S231,
    S231P,
    S231PLegacyHttps,
}

impl Vocabulary {
    const fn namespace_iri(self) -> &'static str {
        match self {
            Self::S231 => "http://data.ashrae.org/S231#",
            Self::S231P => "http://data.ashrae.org/S231P#",
            Self::S231PLegacyHttps => "https://data.ashrae.org/S231P#",
        }
    }
}

const VOCABULARIES: [Vocabulary; 3] = [
    Vocabulary::S231,
    Vocabulary::S231P,
    Vocabulary::S231PLegacyHttps,
];

fn vocabulary_for_namespace(namespace: &str) -> Option<Vocabulary> {
    VOCABULARIES
        .iter()
        .copied()
        .find(|vocabulary| vocabulary.namespace_iri() == namespace)
}

/// Registered local term within a CXF vocabulary namespace.
///
/// The term list anchors OBC specification Table 8.1 and Table 8.2 plus the
/// literal properties observed in pinned producer output. Both
/// `connectedTo` (specification spelling) and `isConnectedTo` (emitter
/// spelling) are registered as separate identities (C-001).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Term {
    // Classes.
    Package,
    Block,
    ElementaryBlock,
    CompositeBlock,
    ExtensionBlock,
    InputConnector,
    OutputConnector,
    BooleanInput,
    BooleanOutput,
    IntegerInput,
    IntegerOutput,
    RealInput,
    RealOutput,
    Parameter,
    Constant,
    EnumerationType,
    DataType,
    Text,
    // Link-valued predicates.
    HasInput,
    HasOutput,
    HasParameter,
    HasConstant,
    HasInstance,
    ContainsBlock,
    ConnectedTo,
    IsConnectedTo,
    IsOfDataType,
    // Literal-valued predicates.
    HasFmuPath,
    Label,
    Description,
    Documentation,
    AccessSpecifier,
    Value,
    Graphics,
    IsFinal,
    IsArray,
    NumberDimensions,
    SizeOfDimensions,
    TranslationSoftware,
    TranslationSoftwareVersion,
}

const TERMS: &[Term] = &[
    Term::Package,
    Term::Block,
    Term::ElementaryBlock,
    Term::CompositeBlock,
    Term::ExtensionBlock,
    Term::InputConnector,
    Term::OutputConnector,
    Term::BooleanInput,
    Term::BooleanOutput,
    Term::IntegerInput,
    Term::IntegerOutput,
    Term::RealInput,
    Term::RealOutput,
    Term::Parameter,
    Term::Constant,
    Term::EnumerationType,
    Term::DataType,
    Term::Text,
    Term::HasInput,
    Term::HasOutput,
    Term::HasParameter,
    Term::HasConstant,
    Term::HasInstance,
    Term::ContainsBlock,
    Term::ConnectedTo,
    Term::IsConnectedTo,
    Term::IsOfDataType,
    Term::HasFmuPath,
    Term::Label,
    Term::Description,
    Term::Documentation,
    Term::AccessSpecifier,
    Term::Value,
    Term::Graphics,
    Term::IsFinal,
    Term::IsArray,
    Term::NumberDimensions,
    Term::SizeOfDimensions,
    Term::TranslationSoftware,
    Term::TranslationSoftwareVersion,
];

impl Term {
    pub(crate) const fn local_name(self) -> &'static str {
        match self {
            Self::Package => "Package",
            Self::Block => "Block",
            Self::ElementaryBlock => "ElementaryBlock",
            Self::CompositeBlock => "CompositeBlock",
            Self::ExtensionBlock => "ExtensionBlock",
            Self::InputConnector => "InputConnector",
            Self::OutputConnector => "OutputConnector",
            Self::BooleanInput => "BooleanInput",
            Self::BooleanOutput => "BooleanOutput",
            Self::IntegerInput => "IntegerInput",
            Self::IntegerOutput => "IntegerOutput",
            Self::RealInput => "RealInput",
            Self::RealOutput => "RealOutput",
            Self::Parameter => "Parameter",
            Self::Constant => "Constant",
            Self::EnumerationType => "EnumerationType",
            Self::DataType => "DataType",
            Self::Text => "String",
            Self::HasInput => "hasInput",
            Self::HasOutput => "hasOutput",
            Self::HasParameter => "hasParameter",
            Self::HasConstant => "hasConstant",
            Self::HasInstance => "hasInstance",
            Self::ContainsBlock => "containsBlock",
            Self::ConnectedTo => "connectedTo",
            Self::IsConnectedTo => "isConnectedTo",
            Self::IsOfDataType => "isOfDataType",
            Self::HasFmuPath => "hasFmuPath",
            Self::Label => "label",
            Self::Description => "description",
            Self::Documentation => "documentation",
            Self::AccessSpecifier => "accessSpecifier",
            Self::Value => "value",
            Self::Graphics => "graphics",
            Self::IsFinal => "isFinal",
            Self::IsArray => "isArray",
            Self::NumberDimensions => "numberDimensions",
            Self::SizeOfDimensions => "sizeOfDimensions",
            Self::TranslationSoftware => "translationSoftware",
            Self::TranslationSoftwareVersion => "translationSoftwareVersion",
        }
    }

    fn from_local_name(local_name: &str) -> Option<Self> {
        TERMS
            .iter()
            .copied()
            .find(|term| term.local_name() == local_name)
    }

    const fn is_link(self) -> bool {
        matches!(
            self,
            Self::HasInput
                | Self::HasOutput
                | Self::HasParameter
                | Self::HasConstant
                | Self::HasInstance
                | Self::ContainsBlock
                | Self::ConnectedTo
                | Self::IsConnectedTo
                | Self::IsOfDataType
        )
    }
}

/// Full internal identity of one recognized term: namespace plus local term.
///
/// `connectedTo` and `isConnectedTo` never share identity (C-001), and
/// vocabulary generations never share identity (C-002, C-016).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct TermId {
    vocabulary: Vocabulary,
    term: Term,
}

impl TermId {
    pub(crate) const fn term(self) -> Term {
        self.term
    }

    pub(crate) const fn namespace_iri(self) -> &'static str {
        self.vocabulary.namespace_iri()
    }
}

/// Prefix spellings activated by the document's own `@context` entries.
#[derive(Default)]
struct ActiveContext {
    prefixes: BTreeMap<Arc<str>, Vocabulary>,
}

fn activate_context(members: &[OrderedMember]) -> ActiveContext {
    let mut active = ActiveContext::default();
    for member in members {
        if &*member.name == "@context" {
            activate_context_value(&member.value, &mut active);
        }
    }
    active
}

fn activate_context_value(value: &OrderedValue, active: &mut ActiveContext) {
    match value {
        OrderedValue::Object { members, .. } => {
            for member in members {
                if let OrderedValue::String {
                    value: namespace, ..
                } = &member.value
                    && let Some(vocabulary) = vocabulary_for_namespace(namespace)
                {
                    active.prefixes.insert(member.name.clone(), vocabulary);
                }
            }
        }
        OrderedValue::Array { values, .. } => {
            for value in values {
                activate_context_value(value, active);
            }
        }
        _ => {}
    }
}

/// Resolves one authored spelling to its registered internal identity.
///
/// Full IRIs always resolve. Compacted spellings resolve only when the
/// document context maps their prefix to the registered namespace IRI;
/// nothing else is expanded (no partial context expander, per D-030).
fn lookup(spelled: &str, context: &ActiveContext) -> Option<TermId> {
    for vocabulary in VOCABULARIES {
        if let Some(local) = spelled.strip_prefix(vocabulary.namespace_iri()) {
            return Term::from_local_name(local).map(|term| TermId { vocabulary, term });
        }
    }
    let (prefix, local) = spelled.split_once(':')?;
    if prefix.is_empty() || local.is_empty() {
        return None;
    }
    let vocabulary = *context.prefixes.get(prefix)?;
    Term::from_local_name(local).map(|term| TermId { vocabulary, term })
}

/// Resolves an authored `isOfDataType` target spelling to a registered
/// primitive datatype under the document's registered context.
fn resolve_data_type(spelling: &str, context: &ActiveContext) -> Option<DataTypeKind> {
    let local = {
        let mut full_iri_match = None;
        for vocabulary in VOCABULARIES {
            if let Some(local) = spelling.strip_prefix(vocabulary.namespace_iri()) {
                full_iri_match = Some(local);
                break;
            }
        }
        match full_iri_match {
            Some(local) => local,
            None => {
                let (prefix, local) = spelling.split_once(':')?;
                context.prefixes.get(prefix)?;
                local
            }
        }
    };
    match local {
        "Real" => Some(DataTypeKind::Real),
        "Integer" => Some(DataTypeKind::Integer),
        "Boolean" => Some(DataTypeKind::Boolean),
        "String" => Some(DataTypeKind::Text),
        _ => None,
    }
}

/// Private diagnostic code produced by the projection layer.
///
/// These codes are crate-private until W-014 stabilizes the CXF validation
/// surface; the owning PR lists them for that later promotion.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ProjectionCode {
    /// Root value is not a JSON object, so node collection cannot start.
    RootNotObject,
    /// Node object has no usable type spelling: `@type` was absent or every
    /// authored shape failed to yield one (C-008 posture covers the former;
    /// malformed shapes additionally leave verbatim extension records).
    WeaklyTypedNode,
    /// Node carries registered class terms of conflicting node classes.
    ConflictingTypes,
    /// Value string matches a known broken-emitter artifact (C-003).
    ValueArtifact,
    /// A link-valued member is not an `@id` reference object.
    MalformedReference,
    /// An `@id` reference spelling matched no node by exact comparison.
    UnresolvedReference,
    /// Two node objects share the same verbatim `@id` spelling.
    DuplicateNodeId,
}

impl ProjectionCode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::RootNotObject => "CXF-P-000",
            Self::WeaklyTypedNode => "CXF-P-001",
            Self::ConflictingTypes => "CXF-P-002",
            Self::ValueArtifact => "CXF-P-003",
            Self::MalformedReference => "CXF-P-004",
            Self::UnresolvedReference => "CXF-P-005",
            Self::DuplicateNodeId => "CXF-P-006",
        }
    }
}

/// One private projection diagnostic with source-token evidence.
#[derive(Debug)]
pub(crate) struct ProjectionDiagnostic {
    code: ProjectionCode,
    node: Option<usize>,
    token: Range<usize>,
    context: Option<Arc<str>>,
}

impl ProjectionDiagnostic {
    pub(crate) const fn code(&self) -> ProjectionCode {
        self.code
    }

    pub(crate) const fn node(&self) -> Option<usize> {
        self.node
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }

    pub(crate) fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }
}

/// A member that never entered the typed CXF surface.
///
/// Unrecognized predicate spellings, recognized predicates with unexpected
/// value shapes, and `graphics` payloads (C-005 posture: opaque, never
/// interpreted) all land here, verbatim.
#[derive(Debug)]
pub(crate) struct ExtensionRecord {
    node: Option<usize>,
    predicate: Arc<str>,
    token: Range<usize>,
    kind: &'static str,
}

impl ExtensionRecord {
    pub(crate) const fn node(&self) -> Option<usize> {
        self.node
    }

    pub(crate) fn predicate(&self) -> &str {
        &self.predicate
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }

    pub(crate) const fn kind(&self) -> &'static str {
        self.kind
    }
}

/// Primitive literal kind retained for opaque values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiteralKind {
    String,
    Number,
    Boolean,
}

/// A CXF value kept opaque: source token bounds plus, when directly
/// available, the decoded string form. Numbers and booleans retain their
/// exact spelling only through the retained source bytes (D-014).
#[derive(Debug)]
pub(crate) enum OpaqueValue {
    Literal {
        kind: LiteralKind,
        text: Option<Arc<str>>,
        token: Range<usize>,
    },
    TypedObject {
        value_text: Option<Arc<str>>,
        type_spelling: Option<Arc<str>>,
        token: Range<usize>,
    },
    OtherObject {
        token: Range<usize>,
    },
    Array {
        token: Range<usize>,
    },
    Null {
        token: Range<usize>,
    },
}

impl OpaqueValue {
    pub(crate) const fn token(&self) -> &Range<usize> {
        match self {
            Self::Literal { token, .. }
            | Self::TypedObject { token, .. }
            | Self::OtherObject { token }
            | Self::Array { token }
            | Self::Null { token } => token,
        }
    }
}

/// Known broken-emitter literal observed by register row C-003.
const EMITTER_VALUE_ARTIFACT: &str = "{ terms: [Array] }";

fn opaque_value(value: &OrderedValue) -> OpaqueValue {
    match value {
        OrderedValue::Null { token } => OpaqueValue::Null {
            token: token.clone(),
        },
        OrderedValue::String { value, token } => OpaqueValue::Literal {
            kind: LiteralKind::String,
            text: Some(value.clone()),
            token: token.clone(),
        },
        OrderedValue::Number { token } => OpaqueValue::Literal {
            kind: LiteralKind::Number,
            text: None,
            token: token.clone(),
        },
        OrderedValue::Boolean { token, .. } => OpaqueValue::Literal {
            kind: LiteralKind::Boolean,
            text: None,
            token: token.clone(),
        },
        OrderedValue::Array { token, .. } => OpaqueValue::Array {
            token: token.clone(),
        },
        OrderedValue::Object { members, token } => {
            let mut value_text = None;
            let mut type_spelling = None;
            let mut is_typed_object = false;
            for member in members {
                match &*member.name {
                    "@value" => {
                        is_typed_object = true;
                        if let OrderedValue::String { value, .. } = &member.value {
                            value_text = Some(value.clone());
                        }
                    }
                    "@type" => {
                        if let OrderedValue::String { value, .. } = &member.value {
                            type_spelling = Some(value.clone());
                        }
                    }
                    _ => {}
                }
            }
            if is_typed_object {
                OpaqueValue::TypedObject {
                    value_text,
                    type_spelling,
                    token: token.clone(),
                }
            } else {
                OpaqueValue::OtherObject {
                    token: token.clone(),
                }
            }
        }
    }
}

fn opaque_artifact_text(value: &OpaqueValue) -> Option<&str> {
    match value {
        OpaqueValue::Literal {
            kind: LiteralKind::String,
            text: Some(text),
            ..
        } => Some(text),
        OpaqueValue::TypedObject {
            value_text: Some(text),
            ..
        } => Some(text),
        _ => None,
    }
}

/// Payload of one recognized literal-valued member.
#[derive(Debug)]
pub(crate) enum PropertyPayload {
    Text(Arc<str>),
    Boolean(bool),
    Unsigned(u64),
    Value(OpaqueValue),
}

/// One recognized literal-valued member with full term identity.
#[derive(Debug)]
pub(crate) struct NodeProperty {
    term: TermId,
    payload: PropertyPayload,
    token: Range<usize>,
}

impl NodeProperty {
    pub(crate) const fn term(&self) -> TermId {
        self.term
    }

    pub(crate) const fn payload(&self) -> &PropertyPayload {
        &self.payload
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }
}

/// Semantic grouping of a link edge. The edge's `TermId` remains the
/// authoritative identity; kinds exist only for graph navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeKind {
    Containment,
    Connection,
    Input,
    Output,
    Parameter,
    Constant,
    DataType,
}

fn edge_kind(term: Term) -> EdgeKind {
    match term {
        Term::HasInstance | Term::ContainsBlock => EdgeKind::Containment,
        Term::ConnectedTo | Term::IsConnectedTo => EdgeKind::Connection,
        Term::HasInput => EdgeKind::Input,
        Term::HasOutput => EdgeKind::Output,
        Term::HasParameter => EdgeKind::Parameter,
        Term::HasConstant => EdgeKind::Constant,
        Term::IsOfDataType => EdgeKind::DataType,
        other => unreachable!("non-link term {other:?} has no edge kind"),
    }
}

/// One link edge from a subject node to a verbatim `@id` reference.
///
/// `target` resolves only by exact string equality of authored spellings;
/// no normalization, prefix expansion, or percent-decoding is applied
/// (C-011). Unresolved references stay verbatim with a diagnostic.
#[derive(Debug)]
pub(crate) struct ProjectionEdge {
    kind: EdgeKind,
    predicate: TermId,
    subject: usize,
    target_spelling: Arc<str>,
    target: Option<usize>,
    data_type: Option<DataTypeKind>,
    token: Range<usize>,
}

impl ProjectionEdge {
    pub(crate) const fn kind(&self) -> EdgeKind {
        self.kind
    }

    pub(crate) const fn predicate(&self) -> TermId {
        self.predicate
    }

    pub(crate) const fn subject(&self) -> usize {
        self.subject
    }

    pub(crate) fn target_spelling(&self) -> &str {
        &self.target_spelling
    }

    pub(crate) const fn target(&self) -> Option<usize> {
        self.target
    }

    pub(crate) const fn data_type(&self) -> Option<DataTypeKind> {
        self.data_type
    }
}

/// Block classification for registered `@type` terms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BlockKind {
    /// The abstract `Block` interface term itself.
    Abstract,
    Elementary,
    Composite,
    Extension,
}

/// Resolved primitive datatype from a registered `isOfDataType` target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DataTypeKind {
    Real,
    Integer,
    Boolean,
    Text,
}

/// Connector direction plus its statically known datatype, if any.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ConnectorClass {
    pub(crate) is_input: bool,
    pub(crate) data_type: Option<DataTypeKind>,
}

/// Node classification derived from registered `@type` spellings.
///
/// `NodeClass` is a *derived structural grouping* (like `EdgeKind`), not a
/// term identity: vocabulary generation and spelling identity stay intact in
/// `type_spellings`, and every predicate/property retains its full `TermId`.
/// Compatible assertions merge to the most specific class; incompatible
/// assertions diagnose and keep the first-authored class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NodeClass {
    Package,
    Block(BlockKind),
    /// Instance typed by a class IRI outside the registered core vocabulary;
    /// the class IRI is preserved verbatim in `type_spellings`.
    LibraryInstance,
    Connector(ConnectorClass),
    Parameter,
    Constant,
    EnumerationType,
    DataType,
    Text,
    /// No usable `@type` spelling (absent, or every authored shape unusable;
    /// C-008 records the common pinned-producer case of nested instances
    /// carrying no type).
    WeakUntyped,
}

fn classify_type(term: Term) -> Option<NodeClass> {
    let class = match term {
        Term::Package => NodeClass::Package,
        Term::Block => NodeClass::Block(BlockKind::Abstract),
        Term::ElementaryBlock => NodeClass::Block(BlockKind::Elementary),
        Term::CompositeBlock => NodeClass::Block(BlockKind::Composite),
        Term::ExtensionBlock => NodeClass::Block(BlockKind::Extension),
        Term::InputConnector => NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: None,
        }),
        Term::OutputConnector => NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: None,
        }),
        Term::BooleanInput => NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: Some(DataTypeKind::Boolean),
        }),
        Term::BooleanOutput => NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: Some(DataTypeKind::Boolean),
        }),
        Term::IntegerInput => NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: Some(DataTypeKind::Integer),
        }),
        Term::IntegerOutput => NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: Some(DataTypeKind::Integer),
        }),
        Term::RealInput => NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: Some(DataTypeKind::Real),
        }),
        Term::RealOutput => NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: Some(DataTypeKind::Real),
        }),
        Term::Parameter => NodeClass::Parameter,
        Term::Constant => NodeClass::Constant,
        Term::EnumerationType => NodeClass::EnumerationType,
        Term::DataType => NodeClass::DataType,
        Term::Text => NodeClass::Text,
        _ => return None,
    };
    Some(class)
}

/// Structural shape classification of an authored `@id` spelling.
///
/// The section 8.5 form (`Package.Class#instance[.child...]`) is recognized
/// only when the whole spelling matches it; emitter identifiers (C-015)
/// remain `Other` and are never split or interpreted lexically.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum IdentifierForm {
    Missing,
    SpecInstance {
        container: Arc<str>,
        steps: Box<[Arc<str>]>,
    },
    Other,
}

fn classify_identifier(spelling: &str) -> IdentifierForm {
    if spelling.contains([':', '/', '\\', ' ', '\t']) {
        return IdentifierForm::Other;
    }
    let Some((container, instance_part)) = spelling.split_once('#') else {
        return IdentifierForm::Other;
    };
    if instance_part.contains('#') || container.is_empty() || instance_part.is_empty() {
        return IdentifierForm::Other;
    }
    let segments: Vec<&str> = instance_part.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return IdentifierForm::Other;
    }
    IdentifierForm::SpecInstance {
        container: Arc::from(container),
        steps: segments.iter().copied().map(Arc::from).collect(),
    }
}

/// One classified CXF graph node with authored-order evidence.
#[derive(Debug)]
pub(crate) struct NodeProjection {
    id_spelling: Option<Arc<str>>,
    id_form: IdentifierForm,
    class: NodeClass,
    type_spellings: Vec<Arc<str>>,
    properties: Vec<NodeProperty>,
    outbound: Vec<usize>,
    inbound: Vec<usize>,
    extensions: Vec<ExtensionRecord>,
    data_type_spelling: Option<Arc<str>>,
    data_type: Option<DataTypeKind>,
    token: Range<usize>,
}

impl NodeProjection {
    pub(crate) fn id_spelling(&self) -> Option<&str> {
        self.id_spelling.as_deref()
    }

    pub(crate) const fn id_form(&self) -> &IdentifierForm {
        &self.id_form
    }

    pub(crate) const fn class(&self) -> NodeClass {
        self.class
    }

    pub(crate) fn type_spellings(&self) -> &[Arc<str>] {
        &self.type_spellings
    }

    pub(crate) fn properties(&self) -> &[NodeProperty] {
        &self.properties
    }

    pub(crate) fn outbound(&self) -> &[usize] {
        &self.outbound
    }

    pub(crate) fn inbound(&self) -> &[usize] {
        &self.inbound
    }

    pub(crate) fn extensions(&self) -> &[ExtensionRecord] {
        &self.extensions
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }

    /// Verbatim first `isOfDataType` reference spelling, if authored and
    /// well formed.
    pub(crate) fn data_type_spelling(&self) -> Option<&str> {
        self.data_type_spelling.as_deref()
    }

    /// Registered primitive datatype resolved from the first well-formed
    /// `isOfDataType` reference, if it names one. Absent means either the
    /// property was never authored (C-008/C-009 posture) or the reference
    /// did not register.
    pub(crate) const fn data_type(&self) -> Option<DataTypeKind> {
        self.data_type
    }

    /// First authored value payload, if any.
    pub(crate) fn value(&self) -> Option<&OpaqueValue> {
        self.properties
            .iter()
            .find_map(|property| match &property.payload {
                PropertyPayload::Value(value) if property.term.term() == Term::Value => Some(value),
                _ => None,
            })
    }

    /// First authored text payload for the given literal term, if any.
    pub(crate) fn text(&self, term: Term) -> Option<&str> {
        self.properties
            .iter()
            .find_map(|property| match &property.payload {
                PropertyPayload::Text(text) if property.term.term() == term => Some(&**text),
                _ => None,
            })
    }

    /// First authored boolean payload for the given literal term, if any.
    pub(crate) fn flag(&self, term: Term) -> Option<bool> {
        self.properties
            .iter()
            .find_map(|property| match &property.payload {
                PropertyPayload::Boolean(flag) if property.term.term() == term => Some(*flag),
                _ => None,
            })
    }
}

/// Aggregate projection counters for later W-022 budget evidence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectionMetrics {
    pub(crate) nodes: u64,
    pub(crate) edges: u64,
    pub(crate) resolved_edges: u64,
    pub(crate) recognized_members: u64,
    pub(crate) extension_members: u64,
    pub(crate) diagnostics: u64,
}

/// Complete private projection of one CXF document.
///
/// The projection owns the admitted source document so every retained token
/// range, extension record, and opaque value stays resolvable for the
/// projection's lifetime (D-014: exact spelling lives only in the retained
/// submitted bytes).
#[derive(Debug)]
pub(crate) struct Projection {
    nodes: Vec<NodeProjection>,
    edges: Vec<ProjectionEdge>,
    diagnostics: Vec<ProjectionDiagnostic>,
    root_extensions: Vec<ExtensionRecord>,
    metrics: ProjectionMetrics,
    source_document: Option<crate::SourceDocument>,
}

impl Projection {
    pub(crate) fn nodes(&self) -> &[NodeProjection] {
        &self.nodes
    }

    pub(crate) fn edges(&self) -> &[ProjectionEdge] {
        &self.edges
    }

    pub(crate) fn diagnostics(&self) -> &[ProjectionDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn root_extensions(&self) -> &[ExtensionRecord] {
        &self.root_extensions
    }

    pub(crate) const fn metrics(&self) -> ProjectionMetrics {
        self.metrics
    }

    /// The admitted source document retained with this projection.
    pub(crate) fn source_document(&self) -> &crate::SourceDocument {
        self.source_document
            .as_ref()
            .expect("projection retains its source document")
    }

    /// Decodes the exact submitted spelling of a retained token range.
    #[cfg(test)]
    pub(crate) fn source_slice(&self, token: &Range<usize>) -> Option<&str> {
        std::str::from_utf8(self.source_document().as_bytes().get(token.clone())?).ok()
    }
}

/// Projects the ordered source view into the private typed CXF record.
pub(crate) fn project(document: OrderedDocument) -> Projection {
    let mut builder = ProjectionBuilder {
        source: document.source_document().as_bytes(),
        ..ProjectionBuilder::default()
    };

    let OrderedValue::Object { members, .. } = document.root() else {
        builder.diagnostic(
            ProjectionCode::RootNotObject,
            None,
            document.root().token().clone(),
            None,
        );
        let mut projection = builder.finish();
        projection.source_document = Some(document.into_source_document());
        return projection;
    };

    builder.context = activate_context(members);

    let mut collector = NodeCollector::default();
    collector.collect_from_members(members);
    // An identity-bearing root is a node even when it also carries `@graph`
    // (named-graph envelope with identity); otherwise its non-keyword
    // members degrade into root-level extension records.
    if looks_like_node(members) {
        collector.nodes.push(document.root());
    } else {
        for member in members {
            if !matches!(&*member.name, "@context" | "@graph" | "@included")
                || structural_member_malformed(&member.value)
            {
                let record = builder.record_extension(
                    None,
                    &member.name,
                    member.value.token().clone(),
                    value_kind(&member.value),
                );
                builder.root_extensions.push(record);
            }
        }
    }

    let root_position = if looks_like_node(members) {
        collector.nodes.len().checked_sub(1)
    } else {
        None
    };
    for (position, node_value) in collector.nodes.into_iter().enumerate() {
        let OrderedValue::Object { members, token } = node_value else {
            continue;
        };
        builder.parse_node(members, token.clone(), root_position == Some(position));
    }

    builder.resolve_edges();
    let mut projection = builder.finish();
    projection.source_document = Some(document.into_source_document());
    projection
}

struct ProjectionBuilder<'a> {
    source: &'a [u8],
    context: ActiveContext,
    nodes: Vec<NodeProjection>,
    edges: Vec<ProjectionEdge>,
    diagnostics: Vec<ProjectionDiagnostic>,
    root_extensions: Vec<ExtensionRecord>,
    id_index: BTreeMap<Arc<str>, usize>,
    recognized_members: u64,
    extension_members: u64,
}

impl Default for ProjectionBuilder<'_> {
    fn default() -> Self {
        Self {
            source: b"",
            context: ActiveContext::default(),
            nodes: Vec::new(),
            edges: Vec::new(),
            diagnostics: Vec::new(),
            root_extensions: Vec::new(),
            id_index: BTreeMap::new(),
            recognized_members: 0,
            extension_members: 0,
        }
    }
}

impl ProjectionBuilder<'_> {
    fn finish(self) -> Projection {
        let resolved_edges = self
            .edges
            .iter()
            .filter(|edge| edge.target.is_some())
            .count() as u64;
        let metrics = ProjectionMetrics {
            nodes: self.nodes.len() as u64,
            edges: self.edges.len() as u64,
            resolved_edges,
            recognized_members: self.recognized_members,
            extension_members: self.extension_members,
            diagnostics: self.diagnostics.len() as u64,
        };
        Projection {
            nodes: self.nodes,
            edges: self.edges,
            diagnostics: self.diagnostics,
            root_extensions: self.root_extensions,
            metrics,
            source_document: None,
        }
    }

    fn diagnostic(
        &mut self,
        code: ProjectionCode,
        node: Option<usize>,
        token: Range<usize>,
        context: Option<Arc<str>>,
    ) {
        self.diagnostics.push(ProjectionDiagnostic {
            code,
            node,
            token,
            context,
        });
    }

    fn record_extension(
        &mut self,
        node: Option<usize>,
        predicate: &Arc<str>,
        token: Range<usize>,
        kind: &'static str,
    ) -> ExtensionRecord {
        self.extension_members += 1;
        ExtensionRecord {
            node,
            predicate: predicate.clone(),
            token,
            kind,
        }
    }

    fn parse_node(&mut self, members: &[OrderedMember], token: Range<usize>, is_root: bool) {
        let index = self.nodes.len();
        let mut id_spelling: Option<Arc<str>> = None;
        let mut type_spellings: Vec<Arc<str>> = Vec::new();
        let mut properties = Vec::new();
        let mut extensions = Vec::new();

        for member in members {
            let name: &str = &member.name;
            match name {
                "@id" => match (&member.value, id_spelling.is_none()) {
                    (OrderedValue::String { value, .. }, true) => {
                        id_spelling = Some(value.clone());
                    }
                    // A non-string or duplicate `@id` cannot supply identity;
                    // the member stays as verbatim extension evidence.
                    _ => {
                        let record = self.record_extension(
                            Some(index),
                            &member.name,
                            member.value.token().clone(),
                            value_kind(&member.value),
                        );
                        extensions.push(record);
                    }
                },
                "@type" => {
                    self.parse_type_member(
                        index,
                        &member.value,
                        &mut type_spellings,
                        &mut extensions,
                    );
                }
                // `@graph`, `@included`, and the root `@context` are consumed
                // structurally (node collection and prefix gating); every
                // other JSON-LD keyword member — including node-scoped
                // contexts the projection deliberately does not apply — is
                // retained verbatim as extension evidence.
                "@graph" | "@included" => {
                    if structural_member_malformed(&member.value) {
                        let record = self.record_extension(
                            Some(index),
                            &member.name,
                            member.value.token().clone(),
                            value_kind(&member.value),
                        );
                        extensions.push(record);
                    }
                }
                "@context" if is_root => {}
                _ if name.starts_with('@') => {
                    let record = self.record_extension(
                        Some(index),
                        &member.name,
                        member.value.token().clone(),
                        value_kind(&member.value),
                    );
                    extensions.push(record);
                }
                _ => match lookup(name, &self.context) {
                    Some(term_id) => {
                        let term = term_id.term();
                        if term == Term::Graphics || !term.is_link() {
                            self.parse_literal(
                                term_id,
                                index,
                                &member.name,
                                &member.value,
                                &mut properties,
                                &mut extensions,
                            );
                        } else {
                            let edge_count = self.edges.len();
                            self.parse_link_value(
                                term_id,
                                index,
                                &member.name,
                                &member.value,
                                &mut extensions,
                            );
                            if self.edges.len() > edge_count {
                                self.recognized_members += 1;
                            }
                        }
                    }
                    None => {
                        let record = self.record_extension(
                            Some(index),
                            &member.name,
                            member.value.token().clone(),
                            value_kind(&member.value),
                        );
                        extensions.push(record);
                    }
                },
            }
        }

        let (class, conflicting) = classify_node(&type_spellings, &self.context);
        if conflicting {
            self.diagnostic(
                ProjectionCode::ConflictingTypes,
                Some(index),
                token.clone(),
                None,
            );
        }
        if class == NodeClass::WeakUntyped {
            self.diagnostic(
                ProjectionCode::WeaklyTypedNode,
                Some(index),
                token.clone(),
                None,
            );
        }

        let id_form = id_spelling
            .as_deref()
            .map_or(IdentifierForm::Missing, classify_identifier);

        if let Some(spelling) = &id_spelling {
            if self.id_index.contains_key(spelling) {
                self.diagnostic(
                    ProjectionCode::DuplicateNodeId,
                    Some(index),
                    token.clone(),
                    None,
                );
            } else {
                self.id_index.insert(spelling.clone(), index);
            }
        }

        self.nodes.push(NodeProjection {
            id_spelling,
            id_form,
            class,
            type_spellings,
            properties,
            outbound: Vec::new(),
            inbound: Vec::new(),
            extensions,
            data_type_spelling: None,
            data_type: None,
            token,
        });
    }

    fn parse_link_value(
        &mut self,
        predicate: TermId,
        subject: usize,
        member_name: &Arc<str>,
        value: &OrderedValue,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        match value {
            OrderedValue::Object { .. } => {
                self.parse_link_object(predicate, subject, member_name, value, extensions);
            }
            OrderedValue::Array { values, .. } => {
                for item in values {
                    if let OrderedValue::Object { .. } = item {
                        self.parse_link_object(predicate, subject, member_name, item, extensions);
                    } else {
                        self.malformed_reference(subject, member_name, item, extensions);
                    }
                }
            }
            _ => self.malformed_reference(subject, member_name, value, extensions),
        }
    }

    fn malformed_reference(
        &mut self,
        subject: usize,
        member_name: &Arc<str>,
        value: &OrderedValue,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        self.diagnostic(
            ProjectionCode::MalformedReference,
            Some(subject),
            value.token().clone(),
            Some(member_name.clone()),
        );
        let record = self.record_extension(
            Some(subject),
            member_name,
            value.token().clone(),
            value_kind(value),
        );
        extensions.push(record);
    }

    fn parse_link_object(
        &mut self,
        predicate: TermId,
        subject: usize,
        member_name: &Arc<str>,
        value: &OrderedValue,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        let OrderedValue::Object { members, token } = value else {
            return;
        };
        // The first string `@id` supplies the reference target. Every other
        // member (including a second `@id` or embedded authored content) is
        // retained verbatim as extension evidence; embedded reference-object
        // content does not itself become a node in C1. When no usable `@id`
        // exists at all, the whole malformed member is recorded once instead.
        let spelling: Option<Arc<str>> = members.iter().find_map(|member| {
            if &*member.name == "@id"
                && let OrderedValue::String { value, .. } = &member.value
            {
                return Some(value.clone());
            }
            None
        });
        let Some(spelling) = spelling else {
            self.malformed_reference(subject, member_name, value, extensions);
            return;
        };
        let mut target_seen = false;
        for member in members {
            if &*member.name == "@id"
                && !target_seen
                && matches!(&member.value, OrderedValue::String { value, .. } if value == &spelling)
            {
                target_seen = true;
                continue;
            }
            let record = self.record_extension(
                Some(subject),
                &member.name,
                member.value.token().clone(),
                value_kind(&member.value),
            );
            extensions.push(record);
        }
        let term = predicate.term();
        let data_type = if term == Term::IsOfDataType {
            resolve_data_type(&spelling, &self.context)
        } else {
            None
        };
        self.edges.push(ProjectionEdge {
            kind: edge_kind(term),
            predicate,
            subject,
            target_spelling: spelling,
            target: None,
            data_type,
            token: token.clone(),
        });
    }

    fn parse_literal(
        &mut self,
        term_id: TermId,
        node: usize,
        member_name: &Arc<str>,
        value: &OrderedValue,
        properties: &mut Vec<NodeProperty>,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        let payload = match expected_shape(term_id.term()) {
            ExpectedPayload::Text => match value {
                OrderedValue::String { value, .. } => Ok(PropertyPayload::Text(value.clone())),
                _ => Err(()),
            },
            ExpectedPayload::Boolean => match value {
                OrderedValue::Boolean { value: flag, .. } => Ok(PropertyPayload::Boolean(*flag)),
                _ => Err(()),
            },
            ExpectedPayload::Unsigned => match value {
                OrderedValue::Number { token } => match parse_u64(self.source, token) {
                    Some(number) => Ok(PropertyPayload::Unsigned(number)),
                    None => Err(()),
                },
                _ => Err(()),
            },
            ExpectedPayload::Opaque => {
                let opaque = opaque_value(value);
                if let Some(text) = opaque_artifact_text(&opaque)
                    && text == EMITTER_VALUE_ARTIFACT
                {
                    self.diagnostic(
                        ProjectionCode::ValueArtifact,
                        Some(node),
                        value.token().clone(),
                        None,
                    );
                }
                Ok(PropertyPayload::Value(opaque))
            }
            ExpectedPayload::ExtensionOnly => Err(()),
        };
        match payload {
            Ok(payload) => {
                self.recognized_members += 1;
                properties.push(NodeProperty {
                    term: term_id,
                    payload,
                    token: value.token().clone(),
                });
            }
            Err(()) => {
                let record = self.record_extension(
                    Some(node),
                    member_name,
                    value.token().clone(),
                    value_kind(value),
                );
                extensions.push(record);
            }
        }
    }

    fn resolve_edges(&mut self) {
        let mut unresolved: Vec<(usize, Arc<str>, Range<usize>)> = Vec::new();
        for edge in &mut self.edges {
            let target = self.id_index.get(&edge.target_spelling).copied();
            // A datatype reference usually names a vocabulary term rather
            // than a graph node; absence from the index is then normal, and
            // presence (for example an in-document enumeration definition)
            // resolves like any other edge.
            if target.is_none() && edge.kind() != EdgeKind::DataType {
                unresolved.push((
                    edge.subject,
                    edge.target_spelling.clone(),
                    edge.token.clone(),
                ));
            }
            edge.target = target;
        }
        for (edge_index, edge) in self.edges.iter().enumerate() {
            self.nodes[edge.subject].outbound.push(edge_index);
            if let Some(target) = edge.target {
                self.nodes[target].inbound.push(edge_index);
            }
            // The verbatim *first* well-formed `isOfDataType` spelling wins,
            // whether or not it registers as a primitive datatype.
            if edge.kind() == EdgeKind::DataType
                && self.nodes[edge.subject].data_type_spelling.is_none()
            {
                self.nodes[edge.subject].data_type_spelling = Some(edge.target_spelling.clone());
                self.nodes[edge.subject].data_type = edge.data_type;
            }
        }
        for (subject, spelling, token) in unresolved {
            self.diagnostic(
                ProjectionCode::UnresolvedReference,
                Some(subject),
                token,
                Some(spelling),
            );
        }
    }
}

enum ExpectedPayload {
    Text,
    Boolean,
    Unsigned,
    Opaque,
    ExtensionOnly,
}

const fn expected_shape(term: Term) -> ExpectedPayload {
    match term {
        Term::HasFmuPath
        | Term::Label
        | Term::Description
        | Term::Documentation
        | Term::AccessSpecifier
        | Term::SizeOfDimensions
        | Term::TranslationSoftware
        | Term::TranslationSoftwareVersion => ExpectedPayload::Text,
        Term::IsFinal | Term::IsArray => ExpectedPayload::Boolean,
        Term::NumberDimensions => ExpectedPayload::Unsigned,
        Term::Value => ExpectedPayload::Opaque,
        // `graphics` payloads stay opaque extension data (C-005 posture);
        // class terms used as predicates and link terms routed here by
        // mistake also stay extension data.
        _ => ExpectedPayload::ExtensionOnly,
    }
}

fn parse_u64(source: &[u8], token: &Range<usize>) -> Option<u64> {
    let text = std::str::from_utf8(source.get(token.clone())?).ok()?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[derive(Default)]
struct NodeCollector<'a> {
    nodes: Vec<&'a OrderedValue>,
}

impl<'a> NodeCollector<'a> {
    fn collect_from_members(&mut self, members: &'a [OrderedMember]) {
        for member in members {
            if matches!(&*member.name, "@graph" | "@included") {
                self.collect_value(&member.value);
            }
        }
    }

    fn collect_value(&mut self, value: &'a OrderedValue) {
        match value {
            OrderedValue::Array { values, .. } => {
                for value in values {
                    self.collect_value(value);
                }
            }
            OrderedValue::Object { .. } => self.collect_node(value),
            _ => {}
        }
    }

    fn collect_node(&mut self, value: &'a OrderedValue) {
        let OrderedValue::Object { members, .. } = value else {
            return;
        };
        // Envelopes: recurse into nested graph members. Only a *pure*
        // structural envelope (nested graph members, no identity, no payload
        // of either keyword or term shape) is itself uncollected; every
        // other object member becomes a node so anonymous and
        // payload-bearing content degrades into weakly typed evidence
        // instead of vanishing.
        let mut has_nested_graph = false;
        for member in members {
            if matches!(&*member.name, "@graph" | "@included") {
                has_nested_graph = true;
                self.collect_value(&member.value);
            }
        }
        let has_payload = members.iter().any(|member| {
            !matches!(&*member.name, "@graph" | "@included")
                || structural_member_malformed(&member.value)
        });
        if looks_like_node(members) || !has_nested_graph || has_payload {
            self.nodes.push(value);
        }
    }
}

/// A `@graph`/`@included` member is consumed structurally only when it has
/// node-carrying shape: an object, or an array containing only objects.
/// Anything else is malformed structural content that must leave extension
/// evidence instead of being silently skipped.
fn structural_member_malformed(value: &OrderedValue) -> bool {
    match value {
        OrderedValue::Object { .. } => false,
        OrderedValue::Array { values, .. } => values
            .iter()
            .any(|item| !matches!(item, OrderedValue::Object { .. })),
        _ => true,
    }
}

fn looks_like_node(members: &[OrderedMember]) -> bool {
    members
        .iter()
        .any(|member| &*member.name == "@id" || &*member.name == "@type")
}

impl ProjectionBuilder<'_> {
    /// Collects authored `@type` spellings: plain strings, arrays, and the
    /// JSON-LD object form `{ "@id": "..." }`. Usable object-form spellings
    /// are salvaged; every other shape stays as verbatim extension evidence
    /// instead of silently dropping or masquerading as a missing type.
    fn parse_type_member(
        &mut self,
        node: usize,
        value: &OrderedValue,
        spellings: &mut Vec<Arc<str>>,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        match value {
            OrderedValue::String { value, .. } => spellings.push(value.clone()),
            OrderedValue::Array { values, .. } => {
                for item in values {
                    self.parse_type_member(node, item, spellings, extensions);
                }
            }
            OrderedValue::Object { members, .. } => {
                let mut salvaged = false;
                for member in members {
                    if !salvaged
                        && &*member.name == "@id"
                        && let OrderedValue::String { value, .. } = &member.value
                    {
                        spellings.push(value.clone());
                        salvaged = true;
                        continue;
                    }
                    let record = self.record_extension(
                        Some(node),
                        &member.name,
                        member.value.token().clone(),
                        value_kind(&member.value),
                    );
                    extensions.push(record);
                }
            }
            _ => {
                let name: Arc<str> = Arc::from("@type");
                let record = self.record_extension(
                    Some(node),
                    &name,
                    value.token().clone(),
                    value_kind(value),
                );
                extensions.push(record);
            }
        }
    }
}

fn classify_node(type_spellings: &[Arc<str>], context: &ActiveContext) -> (NodeClass, bool) {
    let mut best: Option<NodeClass> = None;
    let mut conflicting = false;
    for spelling in type_spellings {
        let Some(class) =
            lookup(spelling, context).and_then(|term_id| classify_type(term_id.term()))
        else {
            continue;
        };
        best = Some(match best {
            None => class,
            Some(current) if current == class => current,
            // Subclass-compatible assertions merge to the more specific
            // registered class (for example `Block` + `CompositeBlock`,
            // `InputConnector` + `RealInput`); genuinely incompatible
            // assertions keep the first-authored class and diagnose.
            Some(current) => match merge_classes(current, class) {
                Some(merged) => merged,
                None => {
                    conflicting = true;
                    current
                }
            },
        });
    }
    match best {
        Some(class) => (class, conflicting),
        None => {
            if type_spellings.is_empty() {
                (NodeClass::WeakUntyped, false)
            } else {
                (NodeClass::LibraryInstance, false)
            }
        }
    }
}

fn class_family_rank(class: NodeClass) -> (u8, u8) {
    match class {
        NodeClass::Package => (0, 0),
        NodeClass::Block(BlockKind::Abstract) => (1, 0),
        NodeClass::Block(_) => (1, 1),
        NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: None,
        }) => (2, 0),
        NodeClass::Connector(ConnectorClass {
            is_input: true,
            data_type: Some(_),
        }) => (2, 1),
        NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: None,
        }) => (3, 0),
        NodeClass::Connector(ConnectorClass {
            is_input: false,
            data_type: Some(_),
        }) => (3, 1),
        NodeClass::Parameter => (4, 0),
        NodeClass::Constant => (5, 0),
        NodeClass::EnumerationType => (6, 0),
        NodeClass::DataType => (7, 0),
        NodeClass::Text => (8, 0),
        NodeClass::LibraryInstance | NodeClass::WeakUntyped => {
            unreachable!("derived classes never come from registered types")
        }
    }
}

fn merge_classes(first: NodeClass, second: NodeClass) -> Option<NodeClass> {
    let (family, first_rank) = class_family_rank(first);
    let (second_family, second_rank) = class_family_rank(second);
    if family != second_family || first_rank == second_rank {
        return None;
    }
    Some(if first_rank > second_rank {
        first
    } else {
        second
    })
}

fn value_kind(value: &OrderedValue) -> &'static str {
    match value {
        OrderedValue::Null { .. } => "null",
        OrderedValue::Boolean { .. } => "boolean",
        OrderedValue::Number { .. } => "number",
        OrderedValue::String { .. } => "string",
        OrderedValue::Array { .. } => "array",
        OrderedValue::Object { .. } => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    macro_rules! project_fixture {
        ($literal:literal) => {
            project_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/projection/",
                $literal
            )))
        };
    }

    fn codes(projection: &Projection) -> Vec<ProjectionCode> {
        projection
            .diagnostics()
            .iter()
            .map(ProjectionDiagnostic::code)
            .collect()
    }

    fn node_by_id<'a>(projection: &'a Projection, id: &str) -> &'a NodeProjection {
        projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some(id))
            .expect("node must exist")
    }

    fn unsigned_property(node: &NodeProjection, term: Term) -> Option<u64> {
        node.properties()
            .iter()
            .find_map(|property| match property.payload() {
                PropertyPayload::Unsigned(value) if property.term().term() == term => Some(*value),
                _ => None,
            })
    }

    #[test]
    fn specform_projects_core_vocabulary() {
        let projection = project_fixture!("cxf-proj-specform.jsonld");
        assert_eq!(projection.nodes().len(), 10);
        assert_eq!(
            codes(&projection),
            Vec::<ProjectionCode>::new(),
            "spec-form document must project without diagnostics"
        );

        let package = node_by_id(&projection, "ExamplePackage");
        assert_eq!(package.class(), NodeClass::Package);
        assert_eq!(
            package.text(Term::TranslationSoftware),
            Some("cxf-json projection tests")
        );
        assert_eq!(
            package.text(Term::TranslationSoftwareVersion),
            Some("0.0.0")
        );

        let sequence = node_by_id(&projection, "ExamplePackage.ExampleSeq");
        assert_eq!(sequence.class(), NodeClass::Block(BlockKind::Composite));
        assert_eq!(sequence.id_form(), &IdentifierForm::Other);
        assert_eq!(sequence.outbound().len(), 4);

        let gain = node_by_id(&projection, "ExamplePackage.ExampleSeq#gain");
        assert_eq!(gain.class(), NodeClass::LibraryInstance);
        assert_eq!(
            gain.type_spellings(),
            &[Arc::from("CDL.Reals.MultiplyByParameter")]
        );
        assert_eq!(
            gain.id_form(),
            &IdentifierForm::SpecInstance {
                container: Arc::from("ExamplePackage.ExampleSeq"),
                steps: Box::from([Arc::from("gain")]),
            }
        );

        let gain_k = node_by_id(&projection, "ExamplePackage.ExampleSeq#gain.k");
        assert_eq!(gain_k.class(), NodeClass::Parameter);
        assert_eq!(gain_k.data_type(), Some(DataTypeKind::Real));
        assert_eq!(gain_k.data_type_spelling(), Some("S231:Real"));
        match gain_k.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::String,
                text: Some(text),
                ..
            }) => assert_eq!(&**text, "100000"),
            other => panic!("gain.k value must stay an opaque string, got {other:?}"),
        }

        let gain_y = node_by_id(&projection, "ExamplePackage.ExampleSeq#gain.y");
        let connection_index = gain_y
            .outbound()
            .iter()
            .copied()
            .find(|index| projection.edges()[*index].kind() == EdgeKind::Connection)
            .expect("gain.y must connect outward");
        let connection = &projection.edges()[connection_index];
        assert_eq!(connection.predicate().term(), Term::ConnectedTo);
        assert_eq!(
            connection.predicate().namespace_iri(),
            "http://data.ashrae.org/S231#"
        );
        let target = connection.target().expect("gain.y connection must resolve");
        assert_eq!(
            projection.nodes()[target].id_spelling(),
            Some("ExamplePackage.ExampleSeq#y")
        );

        let extension = node_by_id(&projection, "ExamplePackage.ExampleSeq#ext");
        assert_eq!(extension.class(), NodeClass::Block(BlockKind::Extension));
        assert_eq!(
            extension.text(Term::HasFmuPath),
            Some("./fmu/heatExchange.fmu")
        );

        let constant = node_by_id(&projection, "ExamplePackage.ExampleSeq#cst");
        assert_eq!(constant.class(), NodeClass::Constant);
        match constant.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::Boolean,
                ..
            }) => {}
            other => panic!("constant value must stay an opaque boolean, got {other:?}"),
        }

        assert_eq!(projection.metrics().edges, 15);
        // Five datatype edges name vocabulary terms rather than graph nodes
        // and therefore stay outside the resolved-edge count; every other
        // edge resolves.
        assert_eq!(projection.metrics().resolved_edges, 10);
        assert!(projection.root_extensions().is_empty());
    }

    #[test]
    fn emitter_layout_projects_and_preserves_authored_order() {
        let projection = project_fixture!("cxf-proj-emitter.jsonld");
        assert_eq!(projection.nodes().len(), 9);

        let gain = node_by_id(&projection, "ex:Project.RootBlock.gain");
        assert_eq!(gain.class(), NodeClass::LibraryInstance);
        assert_eq!(
            gain.type_spellings(),
            &[Arc::from(
                "ex:Buildings.Controls.OBC.CDL.Reals.MultiplyByParameter"
            )]
        );
        assert_eq!(gain.id_form(), &IdentifierForm::Other);
        let graphics = gain
            .extensions()
            .iter()
            .find(|record| record.predicate() == "S231P:graphics")
            .expect("graphics payload must become extension data");
        assert_eq!(graphics.kind(), "string");

        // Weakly typed nested instances keep evidence and diagnose (C-008).
        for child in ["gain.k", "gain.u", "gain.y"] {
            let full = format!("ex:Project.RootBlock.{child}");
            assert_eq!(
                node_by_id(&projection, &full).class(),
                NodeClass::WeakUntyped,
                "{full} must stay weakly typed"
            );
        }
        assert_eq!(
            codes(&projection)
                .iter()
                .filter(|code| **code == ProjectionCode::WeaklyTypedNode)
                .count(),
            3
        );

        // Grouping-property order is exposed exactly as authored (C-010).
        let containment_targets: Vec<&str> = gain
            .outbound()
            .iter()
            .map(|index| &projection.edges()[*index])
            .filter(|edge| edge.kind() == EdgeKind::Containment)
            .map(ProjectionEdge::target_spelling)
            .collect();
        assert_eq!(
            containment_targets,
            [
                "ex:Project.RootBlock.gain.k",
                "ex:Project.RootBlock.gain.u",
                "ex:Project.RootBlock.gain.y"
            ]
        );

        let k = node_by_id(&projection, "ex:Project.RootBlock.k");
        assert_eq!(k.class(), NodeClass::Parameter);
        assert_eq!(k.data_type(), Some(DataTypeKind::Real));
        match k.value() {
            Some(OpaqueValue::TypedObject {
                value_text: Some(text),
                type_spelling: Some(type_iri),
                ..
            }) => {
                assert_eq!(&**text, "0.5");
                assert_eq!(&**type_iri, "xsd:double");
            }
            other => panic!("typed literal value must stay opaque, got {other:?}"),
        }

        let count = node_by_id(&projection, "ex:Project.RootBlock.count");
        match count.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::Number,
                text: None,
                ..
            }) => {}
            other => panic!("numeric values retain spelling only in source bytes, got {other:?}"),
        }

        // The emitter double-links connections in both directions (C-001).
        let inner_u = node_by_id(&projection, "ex:Project.RootBlock.gain.u");
        let outer_u = node_by_id(&projection, "ex:Project.RootBlock.u");
        assert_eq!(inner_u.outbound().len(), 1);
        // One containment edge from `gain` plus one connection edge from
        // the outer connector land on the nested port.
        assert_eq!(inner_u.inbound().len(), 2);
        assert_eq!(outer_u.outbound().len(), 2); // isOfDataType + isConnectedTo

        let metrics = projection.metrics();
        assert_eq!(metrics.nodes, 9);
        assert_eq!(metrics.edges, 13);
        assert_eq!(metrics.resolved_edges, 9);
        assert_eq!(metrics.recognized_members, 22);
        assert_eq!(metrics.extension_members, 1);
        assert_eq!(metrics.diagnostics, 3);
        assert_eq!(
            projection.source_document().as_bytes(),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/projection/cxf-proj-emitter.jsonld"
            ))
        );
    }

    #[test]
    fn connection_predicates_keep_distinct_identity() {
        let spec_projection = project_fixture!("cxf-proj-specform.jsonld");
        let emitter_projection = project_fixture!("cxf-proj-emitter.jsonld");
        let spec_edge = spec_projection
            .edges()
            .iter()
            .find(|edge| edge.kind() == EdgeKind::Connection)
            .expect("spec form uses connectedTo");
        let emitter_edge = emitter_projection
            .edges()
            .iter()
            .find(|edge| edge.kind() == EdgeKind::Connection)
            .expect("emitter form uses isConnectedTo");
        assert_ne!(
            spec_edge.predicate(),
            emitter_edge.predicate(),
            "connectedTo and isConnectedTo must never merge (C-001)"
        );
        assert_eq!(spec_edge.predicate().term(), Term::ConnectedTo);
        assert_eq!(emitter_edge.predicate().term(), Term::IsConnectedTo);
    }

    #[test]
    fn legacy_https_namespace_keeps_distinct_identity() {
        let projection = project_fixture!("cxf-proj-legacy-https.jsonld");
        assert_eq!(projection.nodes().len(), 2);
        assert_eq!(codes(&projection), Vec::<ProjectionCode>::new());

        let input = node_by_id(&projection, "https://example.test/cxf#Loop.u");
        assert_eq!(
            input.class(),
            NodeClass::Connector(ConnectorClass {
                is_input: true,
                data_type: Some(DataTypeKind::Real)
            })
        );
        let connection = projection
            .edges()
            .iter()
            .find(|edge| edge.kind() == EdgeKind::Connection)
            .expect("connection edge must exist");
        assert_eq!(
            connection.predicate().namespace_iri(),
            "https://data.ashrae.org/S231P#"
        );
        let http_identical = lookup(
            "http://data.ashrae.org/S231P#isConnectedTo",
            &ActiveContext::default(),
        )
        .expect("HTTP spelling also registers");
        assert_ne!(
            connection.predicate(),
            http_identical,
            "HTTP and HTTPS namespaces keep distinct identity (C-002)"
        );
        assert!(connection.target().is_some());
    }

    #[test]
    fn unregistered_prefix_terms_fall_to_extensions() {
        let projection = project_str(
            r#"{
              "@context": { "S231P": "https://example.test/not-the-vocabulary#" },
              "@graph": [
                {
                  "@id": "ex:Node",
                  "@type": "S231P:Parameter",
                  "S231P:value": 1
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(node.class(), NodeClass::LibraryInstance);
        assert_eq!(node.extensions().len(), 1);
        assert_eq!(node.extensions()[0].predicate(), "S231P:value");
        assert_eq!(projection.metrics().recognized_members, 0);
        assert_eq!(projection.metrics().extension_members, 1);
    }

    #[test]
    fn weak_enum_parameter_preserves_evidence() {
        let projection = project_fixture!("cxf-proj-weak.jsonld");
        assert_eq!(projection.nodes().len(), 2);

        let missing_value = node_by_id(&projection, "ex:SeriesFan.Controller.venStd");
        assert_eq!(missing_value.class(), NodeClass::WeakUntyped);
        assert!(missing_value.value().is_none());
        assert_eq!(missing_value.text(Term::Label), Some("venStd"));

        let with_value = node_by_id(&projection, "ex:SeriesFan.Controller.freezeStat");
        assert_eq!(with_value.class(), NodeClass::WeakUntyped);
        match with_value.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::String,
                text: Some(text),
                ..
            }) => assert_eq!(&**text, "ex:Types.FreezeProtectionStages.stage1"),
            other => panic!("enumeration literal must be preserved, got {other:?}"),
        }

        assert_eq!(
            codes(&projection)
                .iter()
                .filter(|code| **code == ProjectionCode::WeaklyTypedNode)
                .count(),
            2
        );
    }

    #[test]
    fn value_artifact_is_diagnosed_and_preserved() {
        let projection = project_fixture!("cxf-proj-artifact.jsonld");
        let gain = node_by_id(&projection, "ex:Scaling.gain");
        match gain.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::String,
                text: Some(text),
                ..
            }) => assert_eq!(&**text, "{ terms: [Array] }"),
            other => panic!("artifact value must be preserved verbatim, got {other:?}"),
        }
        assert_eq!(codes(&projection), vec![ProjectionCode::ValueArtifact]);
        assert_eq!(
            projection.diagnostics()[0].node(),
            Some(
                projection
                    .nodes()
                    .iter()
                    .position(|node| node.id_spelling() == Some("ex:Scaling.gain"))
                    .expect("gain node index")
            )
        );

        let typed = project_str(
            r#"{
              "@context": { "S231P": "http://data.ashrae.org/S231P#" },
              "@graph": [
                {
                  "@id": "ex:Q",
                  "@type": "S231P:Parameter",
                  "S231P:value": { "@value": "{ terms: [Array] }", "@type": "xsd:string" }
                }
              ]
            }"#,
        );
        assert_eq!(codes(&typed), vec![ProjectionCode::ValueArtifact]);
    }

    #[test]
    fn encoded_subscript_identifier_stays_verbatim_and_resolves() {
        let projection = project_fixture!("cxf-proj-encoded.jsonld");
        assert_eq!(codes(&projection), Vec::<ProjectionCode>::new());
        let target = node_by_id(&projection, "ex:MultiIn.mulMax.u%5B1%5D");
        assert_eq!(target.text(Term::Label), Some("mulMax.u[1]"));
        assert_eq!(target.id_form(), &IdentifierForm::Other);

        let source = node_by_id(&projection, "ex:MultiIn.u");
        let connection = &projection.edges()[source.outbound()[0]];
        assert_eq!(connection.target_spelling(), "ex:MultiIn.mulMax.u%5B1%5D");
        assert!(connection.target().is_some());
    }

    #[test]
    fn empty_blocks_and_absent_values_are_not_diagnosed() {
        let projection = project_fixture!("cxf-proj-empty.jsonld");
        assert_eq!(codes(&projection), Vec::<ProjectionCode>::new());

        let block = node_by_id(&projection, "ex:Interfaces.RealBus");
        assert_eq!(block.class(), NodeClass::Block(BlockKind::Abstract));
        assert!(block.outbound().is_empty());
        assert!(block.properties().is_empty());

        // A parameter with no authored value and no translation provenance
        // is normal emitter output (C-009, C-013).
        let parameter = node_by_id(&projection, "ex:Interfaces.RealBus.u");
        assert_eq!(parameter.class(), NodeClass::Parameter);
        assert!(parameter.value().is_none());
        assert!(parameter.text(Term::TranslationSoftware).is_none());
        assert_eq!(parameter.data_type(), Some(DataTypeKind::Real));
    }

    #[test]
    fn malformed_unresolved_duplicate_and_conflicting_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:A",
                  "@type": ["S231:Parameter", "S231:Constant"],
                  "S231:hasInput": { "@id": "ex:Gone" },
                  "S231:hasOutput": [ "nope" ],
                  "S231:hasConstant": { "unrelated": 1 }
                },
                { "@id": "ex:A", "@type": "S231:Parameter" }
              ]
            }"#,
        );
        let expected = [
            ProjectionCode::MalformedReference,
            ProjectionCode::MalformedReference,
            ProjectionCode::ConflictingTypes,
            ProjectionCode::DuplicateNodeId,
            ProjectionCode::UnresolvedReference,
        ];
        for expected_code in expected {
            assert!(
                codes(&projection).contains(&expected_code),
                "missing {expected_code:?} in {:?}",
                codes(&projection)
            );
        }
        let node = node_by_id(&projection, "ex:A");
        assert_eq!(
            node.class(),
            NodeClass::Parameter,
            "first registered class wins; conflict is diagnosed"
        );
        assert_eq!(node.extensions().len(), 2);
    }

    #[test]
    fn non_object_root_is_diagnosed() {
        let projection = project_str("[1, 2, 3]");
        assert_eq!(codes(&projection), vec![ProjectionCode::RootNotObject]);
        assert!(projection.nodes().is_empty());
    }

    #[test]
    fn array_metadata_and_flags_project_typedly() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:P.k",
                  "@type": "S231:Parameter",
                  "S231:isArray": true,
                  "S231:numberDimensions": 2,
                  "S231:sizeOfDimensions": "(2,3)",
                  "S231:value": "{i*0.5 +j for i in 1:2, j in 1:3}"
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(node.flag(Term::IsArray), Some(true));
        assert_eq!(unsigned_property(node, Term::NumberDimensions), Some(2));
        assert_eq!(node.text(Term::SizeOfDimensions), Some("(2,3)"));
        match node.value() {
            Some(OpaqueValue::Literal {
                kind: LiteralKind::String,
                text: Some(text),
                ..
            }) => assert_eq!(&**text, "{i*0.5 +j for i in 1:2, j in 1:3}"),
            other => panic!("array expression stays an opaque string, got {other:?}"),
        }
    }

    #[test]
    fn s231_and_s231p_namespaces_stay_distinct() {
        let projection = project_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "S231P": "http://data.ashrae.org/S231P#"
              },
              "@graph": [
                {
                  "@id": "ex:A",
                  "@type": "S231:Parameter",
                  "S231:isOfDataType": { "@id": "S231:Real" }
                },
                {
                  "@id": "ex:B",
                  "@type": "S231P:Parameter",
                  "S231P:isOfDataType": { "@id": "S231P:Real" }
                }
              ]
            }"#,
        );
        let first = projection.edges()[projection.nodes()[0].outbound()[0]].predicate();
        let second = projection.edges()[projection.nodes()[1].outbound()[0]].predicate();
        assert_ne!(
            first, second,
            "vocabulary generations stay distinct (C-016)"
        );
        assert_eq!(
            (first.namespace_iri(), second.namespace_iri()),
            (
                "http://data.ashrae.org/S231#",
                "http://data.ashrae.org/S231P#"
            )
        );
        assert_eq!(projection.nodes()[0].data_type(), Some(DataTypeKind::Real));
        assert_eq!(projection.nodes()[1].data_type(), Some(DataTypeKind::Real));
    }

    #[test]
    fn single_node_document_without_graph_projects() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@id": "ex:Solo",
              "@type": "S231:Block"
            }"#,
        );
        assert_eq!(projection.nodes().len(), 1);
        assert_eq!(
            projection.nodes()[0].class(),
            NodeClass::Block(BlockKind::Abstract)
        );
        assert!(projection.root_extensions().is_empty());
        assert_eq!(codes(&projection), Vec::<ProjectionCode>::new());
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    fn codes(projection: &Projection) -> Vec<ProjectionCode> {
        projection
            .diagnostics()
            .iter()
            .map(ProjectionDiagnostic::code)
            .collect()
    }

    #[test]
    fn embedded_reference_members_are_retained_as_extensions() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#", "ex": "https://example.test/cxf#" },
              "@graph": [
                {
                  "@id": "ex:P",
                  "@type": "S231:Block",
                  "S231:hasInstance": { "@id": "ex:P.c", "S231:label": "embedded label", "@type": "S231:CompositeBlock" }
                },
                { "@id": "ex:P.c", "@type": "S231:CompositeBlock" }
              ]
            }"#,
        );
        let parent = &projection.nodes()[0];
        assert_eq!(
            parent
                .extensions()
                .iter()
                .map(ExtensionRecord::predicate)
                .collect::<Vec<_>>(),
            ["S231:label", "@type"]
        );
        let edge = parent
            .outbound()
            .iter()
            .map(|index| &projection.edges()[*index])
            .find(|edge| edge.kind() == EdgeKind::Containment)
            .expect("containment edge must exist");
        assert!(edge.target().is_some());
    }

    #[test]
    fn anonymous_graph_objects_project_as_weak_nodes() {
        let projection = project_str(
            r#"{
              "@context": { "S231P": "http://data.ashrae.org/S231P#" },
              "@graph": [
                { "S231P:value": 5, "S231P:label": "lo" }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 1);
        let node = &projection.nodes()[0];
        assert_eq!(node.class(), NodeClass::WeakUntyped);
        assert_eq!(node.text(Term::Label), Some("lo"));
        assert!(matches!(
            node.value(),
            Some(OpaqueValue::Literal {
                kind: LiteralKind::Number,
                ..
            })
        ));
        assert_eq!(codes(&projection), vec![ProjectionCode::WeaklyTypedNode]);
    }

    #[test]
    fn root_named_graph_envelope_keeps_its_identity() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@id": "ex:Doc",
              "@type": "S231:Package",
              "@graph": [
                { "@id": "ex:Block", "@type": "S231:Block", "S231:containsBlock": { "@id": "ex:Doc" } }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        assert_eq!(projection.nodes()[1].class(), NodeClass::Package);
        assert_eq!(projection.nodes()[1].id_spelling(), Some("ex:Doc"));
        let edge = projection
            .edges()
            .iter()
            .find(|edge| edge.kind() == EdgeKind::Containment)
            .expect("containment edge must exist");
        assert_eq!(edge.target(), Some(1));
        assert!(projection.root_extensions().is_empty());
    }

    #[test]
    fn datatype_reference_resolves_in_document_targets() {
        let projection = project_str(
            r#"{
              "@context": { "S231P": "http://data.ashrae.org/S231P#", "ex": "https://example.test/cxf#" },
              "@graph": [
                { "@id": "ex:Types.Stage", "@type": "S231P:EnumerationType", "S231P:label": "Stage" },
                {
                  "@id": "ex:Ctl.stage",
                  "@type": "S231P:Parameter",
                  "S231P:isOfDataType": { "@id": "ex:Types.Stage" }
                }
              ]
            }"#,
        );
        let parameter = &projection.nodes()[1];
        assert_eq!(parameter.data_type_spelling(), Some("ex:Types.Stage"));
        assert_eq!(parameter.data_type(), None);
        let edge = &projection.edges()[parameter.outbound()[0]];
        assert_eq!(edge.target(), Some(0));
        assert!(projection.diagnostics().is_empty());
    }

    #[test]
    fn first_authored_datatype_spelling_wins() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:P",
                  "@type": "S231:Parameter",
                  "S231:isOfDataType": [ { "@id": "S231:Custom" }, { "@id": "S231:Real" } ]
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(node.data_type_spelling(), Some("S231:Custom"));
        assert_eq!(node.data_type(), None);
        assert!(
            projection
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code() != ProjectionCode::UnresolvedReference)
        );
    }

    #[test]
    fn object_form_type_spelling_registers() {
        let projection = project_str(
            r#"{
              "@graph": [
                { "@id": "ex:Q", "@type": { "@id": "http://data.ashrae.org/S231P#Parameter" } }
              ]
            }"#,
        );
        assert_eq!(projection.nodes()[0].class(), NodeClass::Parameter);
        assert_eq!(
            projection.nodes()[0].type_spellings(),
            &[Arc::from("http://data.ashrae.org/S231P#Parameter")]
        );
        assert!(projection.diagnostics().is_empty());
    }

    #[test]
    fn nonstring_id_keeps_verbatim_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": 7, "@type": "S231:Parameter" }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(node.class(), NodeClass::Parameter);
        assert_eq!(node.id_form(), &IdentifierForm::Missing);
        assert_eq!(node.extensions().len(), 1);
        assert_eq!(node.extensions()[0].predicate(), "@id");
        assert_eq!(node.extensions()[0].kind(), "number");
        assert!(projection.diagnostics().is_empty());
    }

    #[test]
    fn artifact_matching_is_exact() {
        let projection = project_str(
            r#"{
              "@context": { "S231P": "http://data.ashrae.org/S231P#" },
              "@graph": [
                { "@id": "ex:A", "@type": "S231P:Parameter", "S231P:value": "{ terms: [Array] } " }
              ]
            }"#,
        );
        assert!(
            projection
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code() != ProjectionCode::ValueArtifact),
            "verbatim-distinct strings must not match the C-003 artifact"
        );
    }

    #[test]
    fn subclass_compatible_types_merge_to_most_specific() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:A", "@type": ["S231:Block", "S231:CompositeBlock"] },
                { "@id": "ex:B", "@type": ["S231:InputConnector", "S231:RealInput"] }
              ]
            }"#,
        );
        assert_eq!(
            projection.nodes()[0].class(),
            NodeClass::Block(BlockKind::Composite)
        );
        assert_eq!(
            projection.nodes()[1].class(),
            NodeClass::Connector(ConnectorClass {
                is_input: true,
                data_type: Some(DataTypeKind::Real)
            })
        );
        assert_eq!(codes(&projection), Vec::<ProjectionCode>::new());

        let incompatible = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:C", "@type": ["S231:Parameter", "S231:Constant"] }
              ]
            }"#,
        );
        assert_eq!(codes(&incompatible), vec![ProjectionCode::ConflictingTypes]);
        assert_eq!(incompatible.nodes()[0].class(), NodeClass::Parameter);
    }

    #[test]
    fn projection_retains_source_for_token_spelling() {
        let input = r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:N", "@type": "S231:Parameter", "S231:value": 1e3 }
              ]
            }"#;
        let projection = project_str(input);
        let node = &projection.nodes()[0];
        let value = node.value().expect("value must project");
        assert_eq!(
            projection.source_slice(value.token()),
            Some("1e3"),
            "exact number spelling survives only through the retained source"
        );
        assert_eq!(projection.source_document().as_bytes(), input.as_bytes());
    }
}

#[cfg(test)]
mod keyword_tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    #[test]
    fn unhandled_keyword_members_leave_extension_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                { "@id": "ex:N", "@type": "S231:Parameter", "@language": "en" }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(node.class(), NodeClass::Parameter);
        assert_eq!(node.extensions().len(), 1);
        assert_eq!(node.extensions()[0].predicate(), "@language");
        assert_eq!(projection.metrics().extension_members, 1);
    }

    #[test]
    fn node_scoped_context_leaves_evidence_but_is_never_applied() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:N",
                  "@type": "S231:Parameter",
                  "@context": { "S231P": "http://data.ashrae.org/S231P#" },
                  "S231P:value": "x"
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        assert_eq!(
            node.extensions()
                .iter()
                .map(ExtensionRecord::predicate)
                .collect::<Vec<_>>(),
            ["@context", "S231P:value"],
            "the node-scoped prefix must not register; both members stay verbatim"
        );
        assert!(node.value().is_none());
    }

    #[test]
    fn node_level_included_members_become_nodes() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:N",
                  "@type": "S231:Block",
                  "@included": [ { "@id": "ex:M", "@type": "S231:Parameter" } ]
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        let included = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ex:M"))
            .expect("included content must become a node");
        assert_eq!(included.class(), NodeClass::Parameter);
        assert_eq!(codes(&projection).len(), 0);
    }

    fn codes(projection: &Projection) -> Vec<ProjectionCode> {
        projection
            .diagnostics()
            .iter()
            .map(ProjectionDiagnostic::code)
            .collect()
    }

    #[test]
    fn payload_bearing_envelope_is_collected_as_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@graph": [ { "@id": "ex:Inner", "@type": "S231:Block" } ],
                  "S231:label": "envelope payload"
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        assert_eq!(
            projection.nodes()[1].text(Term::Label),
            Some("envelope payload")
        );
        assert_eq!(projection.nodes()[1].class(), NodeClass::WeakUntyped);
    }

    #[test]
    fn pure_envelope_is_skipped_but_children_survive() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@graph": [ { "@id": "ex:Inner", "@type": "S231:Block" } ]
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 1);
        assert_eq!(projection.nodes()[0].id_spelling(), Some("ex:Inner"));
        assert_eq!(
            projection.nodes()[0].class(),
            NodeClass::Block(BlockKind::Abstract)
        );
        assert_eq!(codes(&projection).len(), 0);
    }
}

#[cfg(test)]
mod seam_tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    #[test]
    fn unconsumed_root_keywords_leave_extension_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@base": "https://example.test/unused-base/",
              "junk": 1,
              "@graph": [ { "@id": "ex:A", "@type": "S231:Block" } ]
            }"#,
        );
        assert_eq!(
            projection
                .root_extensions()
                .iter()
                .map(ExtensionRecord::predicate)
                .collect::<Vec<_>>(),
            ["@base", "junk"]
        );
        assert_eq!(projection.metrics().extension_members, 2);
        assert_eq!(projection.nodes().len(), 1);
    }

    #[test]
    fn malformed_graph_member_shapes_leave_extension_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:N",
                  "@type": "S231:Parameter",
                  "@included": 42
                },
                {
                  "@id": "ex:M",
                  "@type": "S231:Block",
                  "@graph": [ 7 ]
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        for (index, predicate, kind) in [(0, "@included", "number"), (1, "@graph", "array")] {
            assert_eq!(projection.nodes()[index].extensions().len(), 1);
            assert_eq!(
                projection.nodes()[index].extensions()[0].predicate(),
                predicate
            );
            assert_eq!(projection.nodes()[index].extensions()[0].kind(), kind);
        }
    }

    #[test]
    fn envelope_with_keyword_payload_is_collected() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@graph": [ { "@id": "ex:Inner", "@type": "S231:Block" } ],
                  "@base": "https://example.test/never-applied/"
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        let envelope = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling().is_none())
            .expect("payload-bearing envelope must be collected");
        assert_eq!(envelope.class(), NodeClass::WeakUntyped);
        assert_eq!(envelope.extensions()[0].predicate(), "@base");
    }
}

#[cfg(test)]
mod residual_seam_tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    #[test]
    fn malformed_structural_member_on_plain_root_leaves_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@included": 42
            }"#,
        );
        assert!(projection.nodes().is_empty());
        assert_eq!(projection.root_extensions().len(), 1);
        assert_eq!(projection.root_extensions()[0].predicate(), "@included");
        assert_eq!(projection.root_extensions()[0].kind(), "number");
    }

    #[test]
    fn envelope_with_malformed_structural_member_is_collected() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [ { "@graph": 7 } ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 1);
        let envelope = &projection.nodes()[0];
        assert_eq!(envelope.class(), NodeClass::WeakUntyped);
        assert_eq!(envelope.extensions().len(), 1);
        assert_eq!(envelope.extensions()[0].predicate(), "@graph");
    }

    #[test]
    fn anonymous_named_graph_context_is_evidence_not_gating() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@context": { "S231P": "http://data.ashrae.org/S231P#" },
                  "@graph": [ { "@id": "ex:Inner", "S231P:value": 1 } ]
                }
              ]
            }"#,
        );
        assert_eq!(projection.nodes().len(), 2);
        let envelope = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling().is_none())
            .expect("anonymous envelope must be collected");
        assert_eq!(envelope.class(), NodeClass::WeakUntyped);
        assert_eq!(envelope.extensions()[0].predicate(), "@context");
        let inner = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ex:Inner"))
            .expect("inner node must exist");
        assert_eq!(inner.class(), NodeClass::WeakUntyped);
        assert_eq!(
            inner.extensions()[0].predicate(),
            "S231P:value",
            "a node-scoped context must not register its prefix"
        );
    }
}
