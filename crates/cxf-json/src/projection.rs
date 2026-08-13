//! Crate-private W-013-C1/C2 typed CXF projection over the ordered source
//! view.
//!
//! Beyond the section 8.2 core vocabulary, the projection indexes the
//! emitter attribute, unit, metadata, and annotation surface (register rows
//! C-017, C-018): parameter attributes (`start`/`nominal`/`min`/`max`/
//! `fixed`/`instantiate`), unit references (`qudt:hasUnit`,
//! `S231:hasDisplayUnit`, `qudt:hasQuantityKind`) with verbatim,
//! never-normalized target spellings, `graphics` and
//! `conditionalExpression` strings (opaque; C-005/C-006), and the emitter
//! metadata members (`label`, `description`, `documentation`,
//! `accessSpecifier`, `controlledDevice` as text; `defaultValue` as an
//! opaque CXF value; `generatePointlist` as a boolean). `hasFmuPath` is a
//! verbatim text property; both CDL annotation spellings collapse to the
//! `ExtensionBlock` type assertion (C-014 closed).
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
//! remains crate-private; profile 0.1.7 public exports are unchanged.

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
    /// QUDT schema namespace for unit references (C-018). QUDT vocab
    /// namespaces used for unit *targets* are registered separately as
    /// namespace buckets, not term vocabularies.
    QudtSchema,
}

impl Vocabulary {
    const fn namespace_iri(self) -> &'static str {
        match self {
            Self::S231 => "http://data.ashrae.org/S231#",
            Self::S231P => "http://data.ashrae.org/S231P#",
            Self::S231PLegacyHttps => "https://data.ashrae.org/S231P#",
            Self::QudtSchema => "http://qudt.org/schema/qudt#",
        }
    }
}

const VOCABULARIES: [Vocabulary; 4] = [
    Vocabulary::S231,
    Vocabulary::S231P,
    Vocabulary::S231PLegacyHttps,
    Vocabulary::QudtSchema,
];

/// The three S231 generations only; S231-fallback unit targets are
/// classified against these (C-018).
const S231_VOCABULARIES: [Vocabulary; 3] = [
    Vocabulary::S231,
    Vocabulary::S231P,
    Vocabulary::S231PLegacyHttps,
];

impl Vocabulary {
    /// Exact per-identity term registration. The three S231 generations
    /// register the full S231 surface *except* the QUDT unit predicates,
    /// and the QUDT schema namespace registers only its two observed unit
    /// predicates: per-identity allowlists keep registration lexical
    /// instead of a global term×vocabulary cross-product (C-018).
    const fn allows(self, term: Term) -> bool {
        match self {
            Self::S231 | Self::S231P | Self::S231PLegacyHttps => {
                !matches!(term, Term::HasUnit | Term::HasQuantityKind)
            }
            Self::QudtSchema => matches!(term, Term::HasUnit | Term::HasQuantityKind),
        }
    }
}

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
    // C2 attribute and annotation surface (C-017).
    Start,
    Nominal,
    Fixed,
    Instantiate,
    Min,
    Max,
    DefaultValue,
    GeneratePointlist,
    ControlledDevice,
    ConditionalExpression,
    HasDisplayUnit,
    // C2 QUDT unit references (C-018); registered ONLY under the QUDT
    // schema identity via the per-identity allowlist in
    // `Vocabulary::allows` — the emitter never writes them under S231.
    HasUnit,
    HasQuantityKind,
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
    Term::Start,
    Term::Nominal,
    Term::Fixed,
    Term::Instantiate,
    Term::Min,
    Term::Max,
    Term::DefaultValue,
    Term::GeneratePointlist,
    Term::ControlledDevice,
    Term::ConditionalExpression,
    Term::HasDisplayUnit,
    Term::HasUnit,
    Term::HasQuantityKind,
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
            Self::Start => "start",
            Self::Nominal => "nominal",
            Self::Fixed => "fixed",
            Self::Instantiate => "instantiate",
            Self::Min => "min",
            Self::Max => "max",
            Self::DefaultValue => "defaultValue",
            Self::GeneratePointlist => "generatePointlist",
            Self::ControlledDevice => "controlledDevice",
            Self::ConditionalExpression => "conditionalExpression",
            Self::HasDisplayUnit => "hasDisplayUnit",
            Self::HasUnit => "hasUnit",
            Self::HasQuantityKind => "hasQuantityKind",
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
    /// Prefixes mapped to QUDT unit-target namespaces (e.g. the emitter's
    /// `unit` and `q` prefixes; C-018). These activate spelling
    /// classification only; they never register terms. For duplicate
    /// context bindings of the same prefix, both maps keep last-write-wins
    /// order semantics independently.
    unit_prefixes: BTreeMap<Arc<str>, UnitNamespace>,
    /// One record per RETAINED context binding (last-write-wins per
    /// prefix), emitted in prefix-lexicographic order; consumers needing
    /// authored order sort by token. The acceptance-policy rules (W-015)
    /// consume these precisely because activation and observations are
    /// built from this same retained map.
    observations: Vec<NamespaceObservation>,
}

/// Acceptance-matrix classification of one declared context-namespace
/// mapping (W-015). Identity stays distinct per registered IRI; nothing is
/// merged, normalized, or made globally equivalent (C-002/C-016).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NamespaceClass {
    /// `http://data.ashrae.org/S231#`
    S231,
    /// `http://data.ashrae.org/S231P#`
    S231P,
    /// `https://data.ashrae.org/S231P#` (legacy pre-#301 spelling; C-002).
    S231PLegacyHttps,
    /// `http://qudt.org/schema/qudt#`
    QudtSchema,
    /// `http://qudt.org/vocab/unit#`
    QudtUnitVocab,
    /// `http://qudt.org/vocab/quantitykind#`
    QudtQuantityKindVocab,
    /// Anything else, including unregistered variants of the known
    /// vocabulary families (the consumer-facing signal).
    Unregistered,
}

impl NamespaceClass {
    fn from_iri(iri: &str) -> Self {
        if let Some(vocabulary) = vocabulary_for_namespace(iri) {
            return match vocabulary {
                Vocabulary::S231 => Self::S231,
                Vocabulary::S231P => Self::S231P,
                Vocabulary::S231PLegacyHttps => Self::S231PLegacyHttps,
                Vocabulary::QudtSchema => Self::QudtSchema,
            };
        }
        match UnitNamespace::for_iri(iri) {
            Some(UnitNamespace::Unit) => Self::QudtUnitVocab,
            Some(UnitNamespace::QuantityKind) => Self::QudtQuantityKindVocab,
            None => Self::Unregistered,
        }
    }
}

/// One declared `@context` prefix mapping, retained verbatim for the
/// W-015 acceptance-policy rules.
#[derive(Debug)]
pub(crate) struct NamespaceObservation {
    prefix: Arc<str>,
    iri: Arc<str>,
    class: NamespaceClass,
    token: Range<usize>,
}

impl NamespaceObservation {
    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
    }

    pub(crate) fn iri(&self) -> &str {
        &self.iri
    }

    pub(crate) const fn class(&self) -> NamespaceClass {
        self.class
    }

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
    }
}

fn activate_context(members: &[OrderedMember]) -> ActiveContext {
    // Pass 1: collect declared prefix bindings in order. Duplicate bindings
    // of one prefix resolve last-write-wins — the activation and the policy
    // observations must agree on exactly one retained binding, so only the
    // retained one is kept. JSON-LD keyword members (`@base`, `@vocab`,
    // `@language`, …) are not prefix bindings and are excluded.
    let mut bindings: BTreeMap<Arc<str>, (Arc<str>, Range<usize>)> = BTreeMap::new();
    for member in members {
        if &*member.name == "@context" {
            collect_context_bindings(&member.value, &mut bindings);
        }
    }
    // Pass 2: activate terms/unit buckets and record one observation per
    // retained binding — activation and policy can never diverge.
    let mut active = ActiveContext::default();
    for (prefix, (namespace, token)) in bindings {
        if let Some(vocabulary) = vocabulary_for_namespace(&namespace) {
            active.prefixes.insert(prefix.clone(), vocabulary);
        }
        if let Some(bucket) = UnitNamespace::for_iri(&namespace) {
            active.unit_prefixes.insert(prefix.clone(), bucket);
        }
        active.observations.push(NamespaceObservation {
            prefix,
            iri: namespace.clone(),
            class: NamespaceClass::from_iri(&namespace),
            token: token.clone(),
        });
    }
    active
}

fn collect_context_bindings(
    value: &OrderedValue,
    bindings: &mut BTreeMap<Arc<str>, (Arc<str>, Range<usize>)>,
) {
    match value {
        OrderedValue::Object { members, .. } => {
            for member in members {
                if member.name.starts_with('@') {
                    continue;
                }
                if let OrderedValue::String {
                    value: namespace,
                    token,
                } = &member.value
                {
                    bindings.insert(member.name.clone(), (namespace.clone(), token.clone()));
                }
            }
        }
        OrderedValue::Array { values, .. } => {
            for value in values {
                collect_context_bindings(value, bindings);
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
            return Term::from_local_name(local)
                .filter(|term| vocabulary.allows(*term))
                .map(|term| TermId { vocabulary, term });
        }
    }
    let (prefix, local) = spelled.split_once(':')?;
    if prefix.is_empty() || local.is_empty() {
        return None;
    }
    let vocabulary = *context.prefixes.get(prefix)?;
    Term::from_local_name(local)
        .filter(|term| vocabulary.allows(*term))
        .map(|term| TermId { vocabulary, term })
}

/// Resolves an authored `isOfDataType` target spelling to a registered
/// primitive datatype under the document's registered context.
fn resolve_data_type(spelling: &str, context: &ActiveContext) -> Option<DataTypeKind> {
    // Datatype terms only register under S231 generation namespaces; a
    // QUDT-schema spelling like `qudt:Real` is not a CXF datatype.
    let local = {
        let mut full_iri_match = None;
        for vocabulary in S231_VOCABULARIES {
            if let Some(local) = spelling.strip_prefix(vocabulary.namespace_iri()) {
                full_iri_match = Some(local);
                break;
            }
        }
        match full_iri_match {
            Some(local) => local,
            None => {
                let (prefix, local) = spelling.split_once(':')?;
                let vocabulary = context.prefixes.get(prefix)?;
                if !S231_VOCABULARIES.contains(vocabulary) {
                    return None;
                }
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

/// QUDT vocab namespace buckets used by the emitter for unit target
/// spellings (C-018).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnitNamespace {
    /// `http://qudt.org/vocab/unit#`
    Unit,
    /// `http://qudt.org/vocab/quantitykind#`
    QuantityKind,
}

impl UnitNamespace {
    const fn iri(self) -> &'static str {
        match self {
            Self::Unit => "http://qudt.org/vocab/unit#",
            Self::QuantityKind => "http://qudt.org/vocab/quantitykind#",
        }
    }

    fn for_iri(namespace: &str) -> Option<Self> {
        match namespace {
            "http://qudt.org/vocab/unit#" => Some(Self::Unit),
            "http://qudt.org/vocab/quantitykind#" => Some(Self::QuantityKind),
            _ => None,
        }
    }

    /// The emitter declares `unit` and `q` as prefix spellings for these
    /// namespaces; when a compacted spelling's prefix maps here through the
    /// document context, the target class is known by bucket.
    const fn target_class(self) -> UnitTargetClass {
        match self {
            Self::Unit => UnitTargetClass::QudtUnitIri,
            Self::QuantityKind => UnitTargetClass::QudtQuantityKindIri,
        }
    }
}

/// Verb shape of one unit-carrying property.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitRole {
    /// `qudt:hasUnit` (S231 `unit` attribute; C-018).
    Unit,
    /// `S231:hasDisplayUnit`.
    DisplayUnit,
    /// `qudt:hasQuantityKind` (`quantity` attribute).
    QuantityKind,
}

/// Classification of a unit target spelling (never normalized; C-018).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitTargetClass {
    /// Full IRI or context-activated compacted spelling of
    /// `http://qudt.org/vocab/unit#…`.
    QudtUnitIri,
    /// Full IRI or context-activated compacted spelling of
    /// `http://qudt.org/vocab/quantitykind#…`.
    QudtQuantityKindIri,
    /// S231-generation fallback spelling (`S231:<raw>`); the emitter uses
    /// this for units outside its 27-entry mapping table.
    S231Fallback,
    /// Anything else, kept verbatim.
    Other,
}

/// One unit-kind reference retained with its role and verbatim spelling.
#[derive(Debug)]
pub(crate) struct UnitReference {
    role: UnitRole,
    spelling: Arc<str>,
    target_class: UnitTargetClass,
}

impl UnitReference {
    pub(crate) const fn role(&self) -> UnitRole {
        self.role
    }

    pub(crate) fn spelling(&self) -> &str {
        &self.spelling
    }

    pub(crate) const fn target_class(&self) -> UnitTargetClass {
        self.target_class
    }
}

/// Classifies an authored unit target spelling. Full QUDT IRIs always
/// classify; compacted `unit:`/`q:` spellings classify only when the
/// document context maps their prefix to the exact QUDT vocab IRI. S231
/// spellings (full, or compacted through a registered generation prefix)
/// classify as the emitter's unknown-unit fallback. Nothing resolves
/// against QUDT and nothing is rewritten (C-018).
fn classify_unit_target(spelling: &str, context: &ActiveContext) -> UnitTargetClass {
    for bucket in [UnitNamespace::Unit, UnitNamespace::QuantityKind] {
        if spelling.starts_with(bucket.iri()) {
            return bucket.target_class();
        }
    }
    for vocabulary in S231_VOCABULARIES {
        if spelling.starts_with(vocabulary.namespace_iri()) {
            return UnitTargetClass::S231Fallback;
        }
    }
    if let Some((prefix, local)) = spelling.split_once(':')
        && !local.is_empty()
    {
        if let Some(bucket) = context.unit_prefixes.get(prefix) {
            return bucket.target_class();
        }
        // Only S231-generation prefixes are the emitter's unknown-unit
        // fallback shape; a QUDT-schema-prefixed target (or anything else
        // registered but non-S231) is just `Other`.
        if let Some(vocabulary) = context.prefixes.get(prefix)
            && S231_VOCABULARIES.contains(vocabulary)
        {
            return UnitTargetClass::S231Fallback;
        }
    }
    UnitTargetClass::Other
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
/// Unrecognized predicate spellings and recognized predicates with
/// unexpected value shapes land here, verbatim. Since C2, `graphics` string
/// payloads are indexed as text properties (C-005 posture: opaque, never
/// interpreted); only non-string graphics shapes land here.
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
    /// Unit-kind reference with verbatim spelling and target class (C-018).
    Unit(UnitReference),
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

    pub(crate) const fn token(&self) -> &Range<usize> {
        &self.token
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
    namespace_observations: Vec<NamespaceObservation>,
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

    /// The retained root `@context` prefix bindings, verbatim, one per
    /// prefix (prefix-lexicographic order). The W-015 acceptance-policy
    /// rules consume these; the projection itself draws no accept/reject
    /// conclusions.
    pub(crate) fn namespace_observations(&self) -> &[NamespaceObservation] {
        &self.namespace_observations
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
            namespace_observations: self.context.observations,
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

    /// Wrong-shaped unit members (non-reference-object values, objects
    /// without a usable `@id`, and array items of either shape) diagnose
    /// with the same malformed-reference code links use, and leave the
    /// same verbatim extension evidence.
    fn malformed_unit(
        &mut self,
        node: usize,
        member_name: &Arc<str>,
        value: &OrderedValue,
        extensions: &mut Vec<ExtensionRecord>,
    ) {
        self.diagnostic(
            ProjectionCode::MalformedReference,
            Some(node),
            value.token().clone(),
            Some(member_name.clone()),
        );
        let record = self.record_extension(
            Some(node),
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
        // Unit members follow the link contract's array and diagnostics
        // surface: object items index per item, wrong-shaped items
        // (including nested arrays) diagnose as malformed references with
        // extension evidence, and the whole array member counts exactly
        // once toward recognized members when any item indexes — matching
        // `parse_link_value` member-level counting.
        if expected_shape(term_id.term()) == ExpectedPayload::Unit {
            match value {
                OrderedValue::Array { values, .. } => {
                    let before = properties.len();
                    for item in values {
                        if let OrderedValue::Object { .. } = item {
                            self.parse_literal(
                                term_id,
                                node,
                                member_name,
                                item,
                                properties,
                                extensions,
                            );
                        } else {
                            self.malformed_unit(node, member_name, item, extensions);
                        }
                    }
                    let added = properties.len() - before;
                    if added > 0 {
                        self.recognized_members -= (added - 1) as u64;
                    }
                    return;
                }
                OrderedValue::Object { members, .. } => {
                    let spelling: Option<Arc<str>> = members.iter().find_map(|member| {
                        if &*member.name == "@id"
                            && let OrderedValue::String { value, .. } = &member.value
                        {
                            return Some(value.clone());
                        }
                        None
                    });
                    let Some(spelling) = spelling else {
                        self.malformed_unit(node, member_name, value, extensions);
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
                            Some(node),
                            &member.name,
                            member.value.token().clone(),
                            value_kind(&member.value),
                        );
                        extensions.push(record);
                    }
                    self.recognized_members += 1;
                    properties.push(NodeProperty {
                        term: term_id,
                        payload: PropertyPayload::Unit(UnitReference {
                            role: unit_role(term_id.term()),
                            target_class: classify_unit_target(&spelling, &self.context),
                            spelling,
                        }),
                        token: value.token().clone(),
                    });
                    return;
                }
                _ => {
                    self.malformed_unit(node, member_name, value, extensions);
                    return;
                }
            }
        }
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
            // Unit members are fully handled (or malformed-diagnosed) by
            // the contract block above this match.
            ExpectedPayload::Unit => {
                unreachable!("unit members return before payload building")
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum ExpectedPayload {
    Text,
    Boolean,
    Unsigned,
    Opaque,
    Unit,
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
        | Term::ControlledDevice
        // `graphics` is indexed verbatim as an opaque string (C-005): the
        // emitter re-serializes annotation objects into text, and #278
        // shows that text can be syntactically damaged.
        | Term::Graphics
        // Conditional-output expressions arrive as strings that #321 shows
        // can be corrupted (C-006: opaque plus diagnostics).
        | Term::ConditionalExpression
        | Term::TranslationSoftware
        | Term::TranslationSoftwareVersion => ExpectedPayload::Text,
        Term::IsFinal | Term::IsArray | Term::Fixed | Term::GeneratePointlist => {
            ExpectedPayload::Boolean
        }
        Term::NumberDimensions => ExpectedPayload::Unsigned,
        // Attribute values stay fully opaque, including typed literals like
        // the `xsd:decimal` nominal shape (C-017).
        Term::Value
        | Term::Start
        | Term::Nominal
        | Term::Instantiate
        | Term::Min
        | Term::Max
        | Term::DefaultValue => ExpectedPayload::Opaque,
        Term::HasUnit | Term::HasDisplayUnit | Term::HasQuantityKind => {
            ExpectedPayload::Unit
        }
        // Class terms used as predicates and link terms routed here by
        // mistake stay extension data.
        _ => ExpectedPayload::ExtensionOnly,
    }
}

/// Role mapping for the three unit-carrying terms; only called for terms
/// routed to `ExpectedPayload::Unit`.
fn unit_role(term: Term) -> UnitRole {
    match term {
        Term::HasUnit => UnitRole::Unit,
        Term::HasDisplayUnit => UnitRole::DisplayUnit,
        Term::HasQuantityKind => UnitRole::QuantityKind,
        _ => unreachable!("only unit-routed terms reach unit_role"),
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
        // C2: a `graphics` string indexes verbatim as an opaque text
        // property (C-005 posture: retained, never interpreted); only
        // non-string graphics shapes would fall back to extension records.
        let graphics = gain
            .properties()
            .iter()
            .find_map(|property| {
                if property.term().term() == Term::Graphics
                    && let PropertyPayload::Text(text) = property.payload()
                {
                    return Some(text.as_ref());
                }
                None
            })
            .expect("graphics payload must index as verbatim text");
        assert_eq!(
            graphics,
            "Placement(transformation(extent={{-60,-50},{-40,-30}}))"
        );

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
        assert_eq!(metrics.recognized_members, 23);
        assert_eq!(metrics.extension_members, 0);
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

#[cfg(test)]
mod c2_surface_tests {
    use super::*;
    use crate::ParseOptions;

    fn project_str(input: &str) -> Projection {
        let preflight = crate::json::admit_and_preflight(input.as_bytes(), &ParseOptions::new())
            .expect("test document must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    fn units_fixture() -> Projection {
        let bytes = include_bytes!("../tests/projection/cxf-proj-units.jsonld");
        let preflight = crate::json::admit_and_preflight(bytes, &ParseOptions::new())
            .expect("units fixture must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        project(document)
    }

    fn text_property(node: &NodeProjection, term: Term) -> Option<&str> {
        node.properties().iter().find_map(|property| {
            if property.term().term() == term
                && let PropertyPayload::Text(text) = property.payload()
            {
                return Some(text.as_ref());
            }
            None
        })
    }

    fn unit_property(node: &NodeProjection, term: Term) -> Option<&UnitReference> {
        node.properties().iter().find_map(|property| {
            if property.term().term() == term
                && let PropertyPayload::Unit(reference) = property.payload()
            {
                return Some(reference);
            }
            None
        })
    }

    /// C-018: the three unit-carrying predicates index verbatim spellings
    /// with classified targets; the emitter's prefix mapping (`qudt`,
    /// `unit`, `q`) activates only through the document context.
    #[test]
    fn c018_unit_roles_and_target_classes() {
        let projection = units_fixture();
        let kpi = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ExamplePackage.GainParameters.kP"))
            .expect("kP node must exist");

        let unit = unit_property(kpi, Term::HasUnit).expect("qudt:hasUnit indexed");
        assert_eq!(unit.role(), UnitRole::Unit);
        assert_eq!(unit.spelling(), "unit:PA");
        assert_eq!(unit.target_class(), UnitTargetClass::QudtUnitIri);

        let display = unit_property(kpi, Term::HasDisplayUnit).expect("hasDisplayUnit indexed");
        assert_eq!(display.role(), UnitRole::DisplayUnit);
        assert_eq!(display.spelling(), "S231:bar");
        // Compacted S231 spelling = the emitter's unknown-unit fallback
        // shape (not normalized, not resolved against QUDT).
        assert_eq!(display.target_class(), UnitTargetClass::S231Fallback);

        let kind = unit_property(kpi, Term::HasQuantityKind).expect("hasQuantityKind indexed");
        assert_eq!(kind.role(), UnitRole::QuantityKind);
        assert_eq!(kind.spelling(), "q:PressureDifference");
        assert_eq!(kind.target_class(), UnitTargetClass::QudtQuantityKindIri);
    }

    /// Full QUDT IRIs register as predicates and classify as targets
    /// without any context prefix; a full S231 target spelling stays the
    /// emitter's fallback shape.
    #[test]
    fn c018_full_iri_predicates_and_targets() {
        let projection = units_fixture();
        let flow = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ExamplePackage.GainParameters.flow"))
            .expect("flow node must exist");
        let units: Vec<&UnitReference> = flow
            .properties()
            .iter()
            .filter_map(|property| match property.payload() {
                PropertyPayload::Unit(reference) if property.term().term() == Term::HasUnit => {
                    Some(reference)
                }
                _ => None,
            })
            .collect();
        assert_eq!(units.len(), 2, "{:?}", flow.properties());
        assert_eq!(units[0].spelling(), "unit:M3-PER-SEC");
        assert_eq!(units[0].target_class(), UnitTargetClass::QudtUnitIri);
        // `S231:kWh` — the emitter's unspecified-unit fallback spelling.
        assert_eq!(units[1].spelling(), "S231:kWh");
        assert_eq!(units[1].target_class(), UnitTargetClass::S231Fallback);

        let display = unit_property(flow, Term::HasDisplayUnit).expect("full-IRI target indexed");
        assert_eq!(display.spelling(), "http://qudt.org/vocab/unit#M3-PER-SEC");
        assert_eq!(display.target_class(), UnitTargetClass::QudtUnitIri);
        let kind = unit_property(flow, Term::HasQuantityKind).expect("full-IRI target indexed");
        assert_eq!(
            kind.spelling(),
            "http://qudt.org/vocab/quantitykind#Pressure"
        );
        assert_eq!(kind.target_class(), UnitTargetClass::QudtQuantityKindIri);
    }

    /// Exact per-identity registration: `hasUnit`/`hasQuantityKind` only
    /// exist under the QUDT schema identity, never under an S231
    /// generation (C-018). An authored `S231:hasUnit` is extension
    /// evidence, and simple S231 terms never register under `qudt:`.
    #[test]
    fn c018_unit_predicates_are_identity_scoped() {
        let projection = project_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "qudt": "http://qudt.org/schema/qudt#",
                "unit": "http://qudt.org/vocab/unit#",
                "q": "http://qudt.org/vocab/quantitykind#"
              },
              "@graph": [
                {
                  "@id": "ex:t1",
                  "@type": "S231:Parameter",
                  "S231:hasUnit": { "@id": "unit:PA" },
                  "qudt:label": "cross-namespace probe",
                  "qudt:hasQuantityKind": [ { "@id": "q:Angle" }, "bad item", [ { "@id": "q:Time" } ] ]
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        let predicates: Vec<&str> = node
            .extensions()
            .iter()
            .map(|record| record.predicate())
            .collect();
        // `S231:hasUnit` and `qudt:label` are cross-identity probes; the
        // array's wrong-shaped string item also leaves evidence, while its
        // valid reference object still indexes.
        assert!(predicates.contains(&"S231:hasUnit"), "{predicates:?}");
        assert!(predicates.contains(&"qudt:label"), "{predicates:?}");
        let kind = unit_property(node, Term::HasQuantityKind).expect("valid array item indexes");
        assert_eq!(kind.spelling(), "q:Angle");
        assert_eq!(kind.target_class(), UnitTargetClass::QudtQuantityKindIri);
        // Both wrong-shaped array items (the bare string and the nested
        // array) leave verbatim extension evidence and diagnose with the
        // same code malformed link references use.
        assert!(
            node.extensions()
                .iter()
                .any(|record| record.kind() == "string"),
            "{:?}",
            node.extensions()
        );
        assert!(
            node.extensions()
                .iter()
                .any(|record| record.kind() == "array"),
            "{:?}",
            node.extensions()
        );
        assert!(
            projection
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == ProjectionCode::MalformedReference),
            "{:?}",
            projection.diagnostics()
        );
    }

    /// A QUDT-schema-compacted unit *target* (`qudt:KiloGM`) is not a
    /// QUDT vocab IRI and is not an S231 fallback: it classifies as
    /// `Other`. Empty-local spellings and undeclared prefixes also stay
    /// `Other`.
    #[test]
    fn c018_schema_prefixed_and_degenerate_targets_stay_other() {
        let projection = project_str(
            r#"{
              "@context": {
                "S231": "http://data.ashrae.org/S231#",
                "qudt": "http://qudt.org/schema/qudt#",
                "unit": "http://qudt.org/vocab/unit#"
              },
              "@graph": [
                {
                  "@id": "ex:t2",
                  "@type": "S231:Parameter",
                  "qudt:hasUnit": [
                    { "@id": "qudt:KiloGM" },
                    { "@id": "unit:" },
                    { "@id": "vendor:degAPI" }
                  ]
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        let classes: Vec<UnitTargetClass> = node
            .properties()
            .iter()
            .filter_map(|property| match property.payload() {
                PropertyPayload::Unit(reference) => Some(reference.target_class()),
                _ => None,
            })
            .collect();
        assert_eq!(
            classes,
            &[
                UnitTargetClass::Other,
                UnitTargetClass::Other,
                UnitTargetClass::Other
            ]
        );
    }

    /// C-017: emitter attribute members index verbatim; the xsd:decimal
    /// nominal typed literal stays whole (value text and type spelling).
    #[test]
    fn c017_attributes_index_verbatim() {
        let projection = units_fixture();
        let kpi = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ExamplePackage.GainParameters.kP"))
            .expect("kP node must exist");
        assert_eq!(text_property(kpi, Term::AccessSpecifier), Some("public"));
        assert_eq!(
            text_property(kpi, Term::Description),
            Some("Proportional gain of the example")
        );
        assert_eq!(text_property(kpi, Term::Label), Some("kP"));
        let nominal = kpi
            .properties()
            .iter()
            .find_map(|property| {
                if property.term().term() == Term::Nominal
                    && let PropertyPayload::Value(value) = property.payload()
                {
                    return Some(value);
                }
                None
            })
            .expect("nominal must be an opaque value");
        match nominal {
            OpaqueValue::TypedObject {
                value_text: Some(text),
                type_spelling: Some(type_spelling),
                ..
            } => {
                assert_eq!(text.as_ref(), "0.5");
                assert_eq!(
                    type_spelling.as_ref(),
                    "http://www.w3.org/2001/XMLSchema#decimal"
                );
            }
            other => panic!("expected typed-object nominal, got {other:?}"),
        }
        // `instantiate` is opaque (emitter writes booleans); `fixed` false
        // still indexes.
        let flow = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ExamplePackage.GainParameters.flow"))
            .expect("flow node must exist");
        assert!(
            flow.properties()
                .iter()
                .any(|property| property.term().term() == Term::Instantiate)
        );
        assert!(kpi.properties().iter().any(|property| matches!(
            property.payload(),
            PropertyPayload::Boolean(false)
        ) && property.term().term() == Term::Fixed));
    }

    /// Wrong-shaped graphics and unit members retain total extension
    /// fallback; no silent drops (C-005, C-018).
    #[test]
    fn wrong_shapes_stay_extension_evidence() {
        let projection = project_str(
            r#"{
              "@context": { "S231": "http://data.ashrae.org/S231#" },
              "@graph": [
                {
                  "@id": "ex:p1",
                  "@type": "S231:Parameter",
                  "S231:graphics": { "S231:coordinateSystem": [0, 0] },
                  "S231:hasUnit": "Pa",
                  "qudt:hasUnit": { "@id": "unit:PA" },
                  "S231:fixed": "yes"
                }
              ]
            }"#,
        );
        let node = &projection.nodes()[0];
        // Four wrong-shape members, each retained verbatim as extension
        // evidence: graphics as an object, hasUnit as a bare string, the
        // compacted qudt:hasUnit (not registered — the context never maps
        // `qudt`), and `fixed` as a string.
        assert_eq!(node.extensions().len(), 4, "{:?}", node.extensions());
        let predicates: Vec<&str> = node
            .extensions()
            .iter()
            .map(|record| record.predicate())
            .collect();
        assert!(predicates.contains(&"S231:graphics"));
        assert!(predicates.contains(&"S231:hasUnit"));
        assert!(predicates.contains(&"qudt:hasUnit"));
        assert!(predicates.contains(&"S231:fixed"));
    }

    /// Index evidence from the pinned ExtensionBlock reference: FMU path as
    /// text, graphics strings verbatim (including unbalanced parens), the
    /// C-006 garbage guard, and metadata strings (C-017). Dual Extension +
    /// library type merges to the registered Extension class.
    #[test]
    fn annotation_surface_from_pinned_evidence() {
        let bytes = include_bytes!("../tests/projection/cxf-proj-annotation.jsonld");
        let preflight = crate::json::admit_and_preflight(bytes, &ParseOptions::new())
            .expect("annotation fixture must pass preflight");
        let (document, _) = preflight.into_ordered_document();
        let projection = project(document);

        let gain = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling() == Some("ex:ExamplePackage.ExternalLoop.externalGain"))
            .expect("customGain node must exist");
        assert_eq!(gain.class(), NodeClass::Block(BlockKind::Extension));
        assert!(
            gain.type_spellings()
                .iter()
                .any(|spelling| &**spelling == "ex:Vendor.CustomGain")
        );
        assert_eq!(
            text_property(gain, Term::HasFmuPath),
            Some("vendor/externalLoop/gain.fmu")
        );
        assert_eq!(
            text_property(gain, Term::Graphics),
            Some("Placement(transformation(extent={{100,90},{140,130}})))")
        );
        assert_eq!(text_property(gain, Term::ControlledDevice), Some("Heater"));
        assert!(gain.properties().iter().any(|property| matches!(
            property.payload(),
            PropertyPayload::Boolean(true)
        ) && property.term().term()
            == Term::GeneratePointlist));

        let input = projection
            .nodes()
            .iter()
            .find(|node| node.id_spelling().is_some_and(|id| id.ends_with(".u")))
            .expect("u node must exist");
        assert_eq!(
            text_property(input, Term::ConditionalExpression),
            Some("not undefined")
        );
        assert!(
            input
                .properties()
                .iter()
                .any(|property| property.term().term() == Term::DefaultValue)
        );
    }
}
