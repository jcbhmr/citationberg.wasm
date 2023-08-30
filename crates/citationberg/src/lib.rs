//! A library for parsing and formatting CSL styles.

#![deny(missing_docs)]
#![deny(unsafe_code)]

use std::num::NonZeroUsize;
use std::ops::{Deref, Not};

use serde::Deserialize;

use quick_xml::de::Deserializer;
use taxonomy::{
    DateVariable, Kind, Locator, NameVariable, NumberVariable, OtherTerm, Term, Variable,
};

pub mod taxonomy;

const EVENT_BUFFER_SIZE: Option<NonZeroUsize> = NonZeroUsize::new(2048);

/// A boolean in CSL.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Boolean(bool);

impl Deref for Boolean {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Boolean {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let res = String::deserialize(deserializer)?;
        Ok(Self(res.to_ascii_lowercase() == "true"))
    }
}

impl Not for Boolean {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// A positive integer in CSL.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NonNegativeInteger(u32);

impl Deref for NonNegativeInteger {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NonNegativeInteger {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let res = String::deserialize(deserializer)?;
        let res = res.trim().parse().map_err(serde::de::Error::custom)?;
        Ok(Self(res))
    }
}

/// A CSL style.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Style {
    /// The style's metadata.
    pub info: StyleInfo,
    /// The locale used if the user didn't specify one.
    /// Overrides the default locale of the parent style.
    #[serde(rename = "@default-locale")]
    pub default_locale: Option<LocaleCode>,
    /// The CSL version the style is compatible with.
    #[serde(rename = "@version")]
    pub version: String,
    /// The style's formatting rules.
    #[serde(flatten)]
    pub rules: Option<IndependentStyle>,
}

impl Style {
    /// Create a style from an XML file.
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::de::DeError> {
        let style_deserializer = &mut Deserializer::from_str(xml);
        style_deserializer.event_buffer_size(EVENT_BUFFER_SIZE);
        let style = Style::deserialize(style_deserializer)?;
        Ok(style)
    }

    /// Retrieve the link to the parent style for dependent styles.
    pub fn parent_link(&self) -> Option<&InfoLink> {
        self.info
            .link
            .iter()
            .find(|link| link.rel == InfoLinkRel::IndependentParent)
    }

    /// Check if the style is dependent.
    pub fn is_dependent(&self) -> bool {
        self.rules.is_none()
    }
}

/// A style with its own formatting rules.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct IndependentStyle {
    /// How the citations are displayed.
    #[serde(rename = "@class")]
    pub class: StyleClass,
    /// How notes or in-text citations are displayed.
    pub citation: Citation,
    /// How the bibliography is displayed.
    pub bibliography: Option<Bibliography>,
    /// Reusable formatting rules.
    #[serde(rename = "macro", default)]
    pub macros: Vec<CslMacro>,
    /// Override localized strings.
    #[serde(default)]
    pub locale: Vec<InlineLocale>,
    /// Whether to use a hyphen when initializing a name.
    ///
    /// Defaults to `true`.
    #[serde(
        rename = "@initialize-with-hyphen",
        default = "IndependentStyle::default_initialize_with_hyphen"
    )]
    pub initialize_with_hyphen: Boolean,
    /// Specifies how to reformat page ranges.
    #[serde(rename = "@page-range-format")]
    pub page_range_format: Option<PageRangeFormat>,
    /// How to treat the non-dropping name particle when sorting.
    #[serde(rename = "@demote-non-dropping-particle", default)]
    pub demote_non_dropping_particle: DemoteNonDroppingParticle,
    /// Options for the names within.
    #[serde(flatten)]
    pub options: InheritableNameOptions,
}

impl IndependentStyle {
    /// Return the default value for `initialize_with_hyphen`.
    pub const fn default_initialize_with_hyphen() -> Boolean {
        Boolean(true)
    }
}

/// An RFC 1766 language code.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct LocaleCode(pub String);

/// How the citations are displayed.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleClass {
    /// Citations are inlined in the text.
    InText,
    /// Citations are displayed in foot- or endnotes.
    Note,
}

/// How to reformat page ranges.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageRangeFormat {
    /// “321–28”
    /// Aliases: `chicago-15`
    #[serde(alias = "chicago-15")]
    Chicago,
    /// “321–28”
    #[serde(rename = "chicago-16")]
    Chicago16,
    /// “321–328”
    Expanded,
    /// “321–8”
    Minimal,
    /// “321–28”
    MinimalTwo,
}

/// How to treat the non-dropping name particle when sorting.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DemoteNonDroppingParticle {
    /// Treat as part of the first name.
    Never,
    /// Treat as part of the first name except when sorting.
    SortOnly,
    /// Treat as part of the family name.
    #[default]
    DisplayAndSort,
}

/// Citation style metadata
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct StyleInfo {
    /// The authors of the style
    #[serde(rename = "author")]
    #[serde(default)]
    pub authors: Vec<StyleAttribution>,
    /// Contributors to the style
    #[serde(rename = "contributor")]
    #[serde(default)]
    pub contibutors: Vec<StyleAttribution>,
    /// Which format the citations are in.
    #[serde(default)]
    pub category: Vec<StyleCategory>,
    /// Which academic field the style is used in.
    #[serde(default)]
    pub field: Vec<Field>,
    /// A unique identifier for the style. May be a URL or an UUID.
    pub id: String,
    /// The ISSN for the source of the style's publication.
    #[serde(default)]
    pub issn: Vec<String>,
    /// The eISSN for the source of the style's publication.
    pub eissn: Option<String>,
    /// The ISSN-L for the source of the style's publication.
    pub issnl: Option<String>,
    /// Links with more information about the style.
    #[serde(default)]
    pub link: Vec<InfoLink>,
    /// When the style was initially published.
    pub published: Option<Timestamp>,
    /// Under which license the style is published.
    pub rights: Option<License>,
    /// A short description of the style.
    pub summary: Option<LocalString>,
    /// The title of the style.
    pub title: LocalString,
    /// A shortened version of the title.
    pub title_short: Option<LocalString>,
    /// When the style was last updated.
    pub updated: Option<Timestamp>,
}

/// A string annotated with a locale.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct LocalString {
    /// The string's locale.
    #[serde(rename = "@xml:lang")]
    pub lang: Option<LocaleCode>,
    /// The string's value.
    #[serde(rename = "$value", default)]
    pub value: String,
}

/// A person affiliated with the style.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct StyleAttribution {
    /// The person's name.
    pub name: String,
    /// The person's email address.
    pub email: Option<String>,
    /// A URI for the person.
    pub uri: Option<String>,
}

/// Which category this style belongs in.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(untagged)]
pub enum StyleCategory {
    /// Which format the citations are in. May only appear once as a child of `category`.
    CitationFormat {
        /// Which format the citations are in.
        #[serde(rename = "@citation-format")]
        format: CitationFormat,
    },
    /// Which academic field the style is used in. May appear multiple times as a child of `category`.
    Field {
        /// Which academic field the style is used in.
        #[serde(rename = "@field")]
        field: Field,
    },
}

/// What type of in-text citation is used.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CitationFormat {
    /// “… (Doe, 1999)”
    AuthorDate,
    /// “… (Doe)”
    Author,
    /// “… \[1\]”
    Numeric,
    /// “… \[doe99\]”
    Label,
    /// The citation appears as a foot- or endnote.
    Note,
}

/// In which academic field the style is used.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Anthropology,
    Astronomy,
    Biology,
    Botany,
    Chemistry,
    Communications,
    Engineering,
    /// Used for generic styles like Harvard and APA.
    #[serde(rename = "generic-base")]
    GenericBase,
    Geography,
    Geology,
    History,
    Humanities,
    Law,
    Linguistics,
    Literature,
    Math,
    Medicine,
    Philosophy,
    Physics,
    PoliticalScience,
    Psychology,
    Science,
    SocialScience,
    Sociology,
    Theology,
    Zoology,
}

/// A link with more information about the style.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct InfoLink {
    /// The link's URL.
    #[serde(rename = "@href")]
    pub href: String,
    /// How the link relates to the style.
    #[serde(rename = "@rel")]
    pub rel: InfoLinkRel,
    /// A human-readable description of the link.
    #[serde(rename = "$value")]
    pub description: Option<String>,
    /// The link's locale.
    #[serde(rename = "@xml:lang")]
    pub locale: Option<LocaleCode>,
}

/// How a link relates to the style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InfoLinkRel {
    /// Website of the style.
    #[serde(rename = "self")]
    Zelf,
    /// URL from which the style is derived. Must not appear in dependent styles.
    Template,
    /// URL of the style's documentation.
    Documentation,
    /// Parent of a dependent style. Must appear in dependent styles.
    IndependentParent,
}

/// An ISO 8601 chapter 5.4 timestamp.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Timestamp(pub String);

/// A license description.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct License {
    /// The license's name.
    #[serde(rename = "$value")]
    pub name: String,
    /// The license's URL.
    #[serde(rename = "@license")]
    pub license: Option<String>,
    /// The license string's locale.
    #[serde(rename = "@xml:lang")]
    pub lang: Option<LocaleCode>,
}

/// Formatting instructions for in-text or note citations.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Citation {
    /// How items are sorted within the citation.
    pub sort: Option<Sort>,
    /// The citation's formatting rules.
    pub layout: Layout,
    /// Expand names that are ambiguous in short form.
    ///
    /// Default: `false`
    #[serde(rename = "@disambiguate-add-givenname", default)]
    pub disambiguate_add_givenname: Boolean,
    /// When to expand names that are ambiguous in short form.
    #[serde(rename = "@disambiguate-add-givenname-rule")]
    pub givenname_disambiguation_rule: Option<DisambiguationRule>,
    /// Disambiguate by adding more names that would otherwise be hidden by et al.
    ///
    /// Default: `false`
    #[serde(rename = "@disambiguate-add-names", default)]
    pub disambiguate_add_names: Boolean,
    /// Disambiguate by adding an alphabetical suffix to the year.
    ///
    /// Default: `false`
    #[serde(rename = "@disambiguate-add-year-suffix", default)]
    pub disambiguate_add_year_suffix: Boolean,
    /// Group items in cite by name.
    #[serde(rename = "@cite-group-delimiter")]
    pub cite_group_delimiter: Option<String>,
    /// How to collapse cites with similar items.
    #[serde(rename = "@collapse")]
    pub collapse: Option<Collapse>,
    /// Delimiter between year suffixes.
    #[serde(rename = "@year-suffix-delimiter")]
    pub year_suffix_delimiter: Option<String>,
    /// Delimiter after a collapsed cite group.
    #[serde(rename = "@after-collapse-delimiter")]
    pub after_collapse_delimiter: Option<String>,
    /// When near-note-distance is true.
    ///
    /// Default: `5`
    #[serde(
        rename = "@near-note-distance",
        default = "Citation::default_near_note_distance"
    )]
    pub near_note_distance: NonNegativeInteger,
    /// Options for the names within.
    #[serde(flatten)]
    pub name_options: InheritableNameOptions,
}

impl Citation {
    /// Return the default value for `cite_group_delimiter` if implicitly needed
    /// due to presence of a `collapse` attribute.
    pub const DEFAULT_CITE_GROUP_DELIMITER: &str = ", ";

    /// Return the `year_suffix_delimiter`.
    pub fn get_year_suffix_delimiter(&self) -> &str {
        self.year_suffix_delimiter
            .as_deref()
            .or(self.layout.delimiter.as_deref())
            .unwrap_or_default()
    }

    /// Return the `after_collapse_delimiter`.
    pub fn get_after_collapse_delimiter(&self) -> &str {
        self.after_collapse_delimiter
            .as_deref()
            .or(self.layout.delimiter.as_deref())
            .unwrap_or_default()
    }

    /// Return the default `near_note_distance`.
    pub const fn default_near_note_distance() -> NonNegativeInteger {
        NonNegativeInteger(5)
    }
}

/// When to expand names that are ambiguous in short form.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisambiguationRule {
    /// Expand to disambiguate both cites and names.
    AllNames,
    /// Expand to disambiguate cites and names but only use initials.
    AllNamesWithInitials,
    /// Same as `AllNames` but only disambiguate the first person in a citation.
    PrimaryName,
    /// Same as `AllNamesWithInitials` but only disambiguate the first person in a citation.
    PrimaryNameWithInitials,
    /// Expand to disambiguate cites but not names.
    #[default]
    ByCite,
}

/// How to collapse cites with similar items.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Collapse {
    /// Collapse items with increasing ranges for numeric styles.
    CitationNumber,
    /// Collapse items with the same authors and different years by omitting the author.
    Year,
    /// Same as `Year`, but equal years are omitted as well.
    YearSuffix,
    /// Same as `YearSuffix`, but also collapse the suffixes into a range.
    YearSuffixRanged,
}

/// Formatting instructions for the bibliography.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Bibliography {
    /// How items are sorted within the citation.
    pub sort: Option<Sort>,
    /// The citation's formatting rules.
    pub layout: Layout,
    /// Render the bibliography in a hanging indent.
    ///
    /// Default: `false`
    #[serde(rename = "@hanging-indent", default)]
    pub hanging_indent: Boolean,
    /// When set, the second field is aligned.
    #[serde(rename = "@second-field-align")]
    pub second_field_align: Option<SecondFieldAlign>,
    /// When set, subsequent identical names are replaced with this.
    #[serde(rename = "@subsequent-author-substitute")]
    pub subsequent_author_substitute: Option<String>,
    /// How to replace subsequent identical names.
    #[serde(rename = "@subsequent-author-substitute-rule", default)]
    pub subsequent_author_substitute_rule: SubsequentAuthorSubstituteRule,
    /// Options for the names within.
    #[serde(flatten)]
    pub options: InheritableNameOptions,
}

/// How to position the first field if the second field is aligned in a bibliography.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecondFieldAlign {
    /// Put the first field in the margin and align with the margin.
    Margin,
    /// Flush the first field with the margin.
    Flush,
}

/// How to replace subsequent identical names in a bibliography.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubsequentAuthorSubstituteRule {
    /// When all names match, replace.
    #[default]
    CompleteAll,
    /// When all names match, replace each name.
    CompleteEach,
    /// Each maching name is replaced.
    PartialEach,
    /// Only the first matching name is replaced.
    PartialFirst,
}

/// How to sort elements in a bibliography or citation.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Sort {
    /// The ordered list of sorting keys.
    #[serde(rename = "key")]
    pub keys: Vec<SortKey>,
}

/// A sorting key.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(untagged)]
pub enum SortKey {
    /// Sort by the value of a variable.
    Variable {
        /// The variable to sort by.
        #[serde(rename = "@variable")]
        variable: Variable,
        /// In which direction to sort.
        #[serde(rename = "@sort", default)]
        sort_direction: SortDirection,
    },
    /// Sort by the output of a macro.
    MacroName {
        /// The name of the macro.
        #[serde(rename = "@macro")]
        name: String,
        /// Override `[InheritedNameOptions::et_al_min]` and
        /// `[InheritedNameOptions::et_al_subsequent_min]` for macros.
        #[serde(rename = "@names-min")]
        names_min: Option<NonNegativeInteger>,
        /// Override `[InheritedNameOptions::et_al_use_first]` and
        /// `[InheritedNameOptions::et_al_subsequent_use_first]` for macros.
        #[serde(rename = "@names-use-first")]
        names_use_first: Option<NonNegativeInteger>,
        /// Override `[InheritedNameOptions::et_al_use_last]` for macros.
        #[serde(rename = "@names-use-last")]
        names_use_last: Option<Boolean>,
        /// In which direction to sort.
        #[serde(rename = "@sort", default)]
        sort_direction: SortDirection,
    },
}

impl From<Variable> for SortKey {
    fn from(value: Variable) -> Self {
        Self::Variable {
            variable: value,
            sort_direction: SortDirection::default(),
        }
    }
}

impl SortKey {
    /// Retrieve the sort direction.
    pub const fn sort_direction(&self) -> SortDirection {
        match self {
            Self::Variable { sort_direction, .. } => *sort_direction,
            Self::MacroName { sort_direction, .. } => *sort_direction,
        }
    }
}

/// The direction to sort in.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortDirection {
    /// Sort in ascending order.
    #[default]
    Ascending,
    /// Sort in descending order.
    Descending,
}

/// A formatting rule.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Layout {
    /// Parts of the rule.
    #[serde(rename = "$value")]
    pub elements: Vec<LayoutRenderingElement>,
    // TODO: Roll into proc-macro because #[serde(flatten)] doesn't work with
    // $value fields.
    /// Set the formatting style.
    // #[serde(flatten)]
    // pub formatting: Formatting,
    // /// Add prefix and suffix.
    // #[serde(flatten)]
    // pub affixes: Affixes,
    /// Delimit pieces of the output.
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
}

/// Possible parts of a formatting rule.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutRenderingElement {
    /// Insert a term or variable.
    Text(Text),
    /// Format a date.
    Date(Date),
    /// Format a number.
    Number(Number),
    /// Format a list of names.
    Names(Names),
    /// Prints a label for a variable.
    Label(Label),
    /// Container for rendering elements.
    Group(Group),
    /// Conditional rendering.
    Choose(Choose),
}

/// Rendering elements.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(untagged)]
pub enum RenderingElement {
    /// A layout element.
    Layout(Layout),
    /// Other rendering elements.
    Other(LayoutRenderingElement),
}

/// Print a term or variable.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Text {
    /// The term or variable to print.
    #[serde(flatten)]
    pub target: TextTarget,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Set layout level.
    #[serde(rename = "@display")]
    pub display: Option<Display>,
    /// Whether to wrap this text in quotes.
    ///
    /// Default: `false`
    #[serde(rename = "@quotes", default)]
    pub quotes: Boolean,
    /// Remove periods from the output.
    ///
    /// Default: `false`
    #[serde(rename = "@strip-periods", default)]
    pub strip_periods: Boolean,
    /// Transform the text case.
    #[serde(rename = "@text-case")]
    pub text_case: Option<TextCase>,
}

/// Various kinds of text targets.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub enum TextTarget {
    /// Prints the value of a variable.
    #[serde(rename = "@variable")]
    Variable(Variable),
    /// Prints the text output of a macro.
    #[serde(rename = "@macro")]
    Macro(String),
    /// Prints a localized term.
    #[serde(rename = "@term")]
    Term(Term),
    /// Prints a given string.
    #[serde(rename = "@value")]
    Value(String),
}

impl From<Variable> for TextTarget {
    fn from(value: Variable) -> Self {
        Self::Variable(value)
    }
}

impl From<Term> for TextTarget {
    fn from(value: Term) -> Self {
        Self::Term(value)
    }
}

/// Formats a date.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Date {
    /// The date to format.
    #[serde(rename = "@variable")]
    pub variable: Variable,
    /// How the localized date should be formatted.
    #[serde(rename = "@form")]
    pub form: Option<DateForm>,
    /// Which parts of the localized date should be included.
    #[serde(rename = "@date-parts")]
    pub parts: Option<DateParts>,
    /// Override the default date parts. Also specifies the order of the parts
    /// if `form` is `None`.
    #[serde(default)]
    pub date_part: Vec<DatePart>,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix. Ignored when this defines a localized date format.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Delimit pieces of the output. Ignored when this defines a localized date format.
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    /// Set layout level.
    #[serde(rename = "@display")]
    pub display: Option<Display>,
    /// Transform the text case.
    #[serde(rename = "@text-case")]
    pub text_case: Option<TextCase>,
}

/// Localized date formats.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    /// “12-15-2005”
    Numeric,
    /// “December 15, 2005”
    Text,
}

/// Which parts of a date should be included.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[allow(missing_docs)]
#[serde(rename_all = "kebab-case")]
pub enum DateParts {
    Year,
    YearMonth,
    #[default]
    YearMonthDay,
}

/// Override the default date parts.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct DatePart {
    /// Kind of the date part.
    #[serde(rename = "@name")]
    pub name: DatePartName,
    /// Form of the date part.
    #[serde(rename = "@form")]
    form: Option<DateAnyForm>,
    /// The string used to delimit two date parts.
    #[serde(rename = "@range-delimiter")]
    pub range_delimiter: Option<String>,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix. Ignored when this defines a localized date format.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Remove periods from the date part.
    ///
    /// Default: `false`
    #[serde(rename = "@strip-periods", default)]
    pub strip_periods: Boolean,
    /// Transform the text case.
    #[serde(rename = "@text-case")]
    pub text_case: Option<TextCase>,
}

impl DatePart {
    /// Retrieve the default delimiter for the date part.
    pub const DEFAULT_DELIMITER: &str = "–";

    /// Retrieve the form.
    pub fn form(&self) -> Option<DateStrongAnyForm> {
        DateStrongAnyForm::for_name(self.name, self.form?)
    }
}

/// The kind of a date part with its `form` attribute.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DatePartName {
    Day,
    Month,
    Year,
}

/// Any allowable date part format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateAnyForm {
    /// “1”
    Numeric,
    /// “01”
    NumericLeadingZeros,
    /// “1st”
    Ordinal,
    /// “January”
    Long,
    /// “Jan.”
    Short,
}

/// Strongly typed date part formats.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DateStrongAnyForm {
    Day(DateDayForm),
    Month(DateMonthForm),
    Year(DateYearForm),
}

impl DateStrongAnyForm {
    /// Get a strongly typed date form for a name. Must return `Some` for valid
    /// CSL files.
    pub fn for_name(name: DatePartName, form: DateAnyForm) -> Option<Self> {
        Some(match name {
            DatePartName::Day => Self::Day(form.form_for_day()?),
            DatePartName::Month => Self::Month(form.form_for_month()?),
            DatePartName::Year => Self::Year(form.form_for_year()?),
        })
    }
}

impl DateAnyForm {
    /// Retrieve the form for a day.
    pub fn form_for_day(&self) -> Option<DateDayForm> {
        match self {
            Self::Numeric => Some(DateDayForm::Numeric),
            Self::NumericLeadingZeros => Some(DateDayForm::NumericLeadingZeros),
            Self::Ordinal => Some(DateDayForm::Ordinal),
            _ => None,
        }
    }

    /// Retrieve the form for a month.
    pub fn form_for_month(&self) -> Option<DateMonthForm> {
        match self {
            Self::Long => Some(DateMonthForm::Long),
            Self::Short => Some(DateMonthForm::Short),
            Self::Numeric => Some(DateMonthForm::Numeric),
            Self::NumericLeadingZeros => Some(DateMonthForm::NumericLeadingZeros),
            _ => None,
        }
    }

    /// Retrieve the form for a year.
    pub fn form_for_year(&self) -> Option<DateYearForm> {
        match self {
            Self::Long => Some(DateYearForm::Long),
            Self::Short => Some(DateYearForm::Short),
            _ => None,
        }
    }
}

/// How a day is formatted.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateDayForm {
    /// “1”
    Numeric,
    /// “01”
    NumericLeadingZeros,
    /// “1st”
    Ordinal,
}

/// How a month is formatted.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateMonthForm {
    /// “January”
    Long,
    /// “Jan.”
    Short,
    /// “1”
    Numeric,
    /// “01”
    NumericLeadingZeros,
}

/// How a year is formatted.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateYearForm {
    /// “2005”
    Long,
    /// “05”
    Short,
}

/// Renders a number.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Number {
    /// The variable whose value is used.
    #[serde(rename = "@variable")]
    pub variable: NumberVariable,
    /// How the number is formatted.
    #[serde(rename = "@form", default)]
    pub form: NumberForm,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Set layout level.
    #[serde(rename = "@display")]
    pub display: Option<Display>,
    /// Transform the text case.
    #[serde(rename = "@text-case")]
    pub text_case: Option<TextCase>,
}

/// How a number is formatted.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NumberForm {
    /// “1”
    #[default]
    Numeric,
    /// “1st”
    Ordinal,
    /// “first”
    LongOrdinal,
    /// “I”
    Roman,
}

/// Renders a list of names.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Names {
    /// The variable whose value is used.
    #[serde(rename = "@variable")]
    pub variable: Vec<NameVariable>,
    /// How the names are formatted.
    pub name: Option<Name>,
    /// Configuration of the et al. abbreviation.
    pub et_al: Option<EtAl>,
    /// Substitutions in case the variable is empty.
    pub substitute: Option<Substitute>,
    /// Label for the names.
    pub label: Option<VariablelessLabel>,
    /// Delimiter between names.
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    /// Options for the names within.
    #[serde(flatten)]
    pub options: InheritableNameOptions,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Set layout level.
    #[serde(rename = "@display")]
    pub display: Option<Display>,
}

/// Configuration of how to print names.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Name {
    /// Delimiter between names.
    #[serde(rename = "@delimiter")]
    pub delimiter: String,
    /// Which name parts to display for personal names.
    #[serde(rename = "@form", default)]
    pub form: NameForm,
    /// Name parts for formatting for the given and family name.
    #[serde(rename = "name-part")]
    pub parts: Vec<NamePart>,
    /// Options for this name.
    #[serde(flatten)]
    pub options: InheritableNameOptions,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
}

impl Default for Name {
    fn default() -> Self {
        Self {
            delimiter: ", ".to_string(),
            form: NameForm::default(),
            parts: Vec::default(),
            options: InheritableNameOptions::default(),
            formatting: Formatting::default(),
            affixes: Affixes::default(),
        }
    }
}

/// Global configuration of how to print names.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(default)]
pub struct InheritableNameOptions {
    /// Delimiter between second-to-last and last name.
    #[serde(rename = "@and")]
    pub and: Option<NameAnd>,
    /// Delimiter inherited to `cs:name` elements.
    #[serde(rename = "@name-delimiter")]
    pub name_delimiter: Option<String>,
    /// Delimiter inherited to `cs:names` elements.
    #[serde(rename = "@names-delimiter")]
    pub names_delimiter: Option<String>,
    /// Delimiter before et al.
    #[serde(rename = "@delimiter-precedes-et-al")]
    pub delimiter_precedes_et_al: DelimiterBehavior,
    /// Whether to use the delimiter before the last name.
    #[serde(rename = "@delimiter-precedes-last")]
    pub delimiter_precedes_last: DelimiterBehavior,
    /// Minimum number of names to use et al.
    #[serde(rename = "@et-al-min")]
    pub et_al_min: Option<NonNegativeInteger>,
    /// Maximum number of names to use before et al.
    #[serde(rename = "@et-al-use-first")]
    pub et_al_use_first: Option<NonNegativeInteger>,
    /// Minimum number of names to use et al. for repeated citations.
    #[serde(rename = "@et-al-subsequent-min")]
    pub et_al_subsequent_min: Option<NonNegativeInteger>,
    /// Maximum number of names to use before et al. for repeated citations.
    #[serde(rename = "@et-al-subsequent-use-first")]
    pub et_al_subsequent_use_first: Option<NonNegativeInteger>,
    /// Whether to use the last name in the author list when there are at least
    /// `et_al_min` names.
    #[serde(rename = "@et-al-use-last")]
    pub et_al_use_last: Boolean,
    /// Which name parts to display for personal names.
    #[serde(rename = "@name-form")]
    pub name_form: NameForm,
    /// Whether to initialize the first name if `initialize-with` is Some.
    #[serde(rename = "@initialize")]
    pub initialize: Boolean,
    /// String to initialize the first name with.
    #[serde(rename = "@initialize-with")]
    pub initialize_with: Option<String>,
    /// Whether to turn the name around.
    #[serde(rename = "@name-as-sort-order")]
    pub name_as_sort_order: Option<NameAsSortOrder>,
    /// Delimiter between given name and first name. Only used if
    /// `name-as-sort-order` is Some.
    #[serde(rename = "@sort-separator")]
    pub sort_separator: String,
}

impl Default for InheritableNameOptions {
    fn default() -> Self {
        Self {
            and: None,
            name_delimiter: None,
            names_delimiter: None,
            delimiter_precedes_et_al: DelimiterBehavior::default(),
            delimiter_precedes_last: DelimiterBehavior::default(),
            et_al_min: None,
            et_al_use_first: None,
            et_al_subsequent_min: None,
            et_al_subsequent_use_first: None,
            et_al_use_last: Boolean::default(),
            name_form: NameForm::default(),
            initialize: Boolean::default(),
            initialize_with: None,
            name_as_sort_order: None,
            sort_separator: ",".to_string(),
        }
    }
}

/// How to render the delimiter before the last name.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NameAnd {
    /// Use the string "and".
    Text,
    /// Use the ampersand character.
    Symbol,
}

/// When delimiters shall be inserted.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterBehavior {
    /// Only used for lists with more than one (`-precedes-et-al`) or two
    /// (`-precedes-last`) names.
    #[default]
    Contextual,
    /// Only use if the preceeding name is inverted (per `name-as-sort-order`).
    AfterInvertedName,
    /// Always use the delimiter for this condition.
    Always,
    /// Never use the delimiter for this condition.
    Never,
}

/// How many name parts to print.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NameForm {
    /// Print all name parts
    #[default]
    Long,
    /// Print only the family name part and non-dropping-particle.
    Short,
    /// Count the total number of names.
    Count,
}

/// In which order to print the names.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NameAsSortOrder {
    /// Only the first name is turned around.
    First,
    /// All names are turned around.
    All,
}

/// How to format a given name part.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NamePart {
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Transform the text case.
    #[serde(flatten)]
    pub text_case: Option<TextCase>,
}

/// Configure the et al. abbreviation.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct EtAl {
    /// Which term to use.
    #[serde(rename = "@term", default)]
    pub term: EtAlTerm,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// Which term to use for et al.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub enum EtAlTerm {
    /// “et al.”
    #[default]
    #[serde(rename = "et al", alias = "et-al")]
    EtAl,
    /// “and others”
    #[serde(rename = "and others", alias = "and-others")]
    AndOthers,
}

impl From<EtAlTerm> for Term {
    fn from(term: EtAlTerm) -> Self {
        match term {
            EtAlTerm::EtAl => Term::Other(OtherTerm::EtAl),
            EtAlTerm::AndOthers => Term::Other(OtherTerm::AndOthers),
        }
    }
}

/// What to do if the name variable is empty.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Substitute {
    /// The layout to use instead.
    #[serde(rename = "$value")]
    pub children: Vec<LayoutRenderingElement>,
}

/// Print a label for a number variable.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Label {
    /// The variable for which to print the label.
    #[serde(rename = "@variable")]
    pub variable: NumberVariable,
    /// The form of the label.
    #[serde(flatten)]
    pub label: VariablelessLabel,
}

/// A label without its variable.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct VariablelessLabel {
    /// What variant of label is chosen.
    #[serde(rename = "@form", default)]
    pub form: TermForm,
    /// How to pluiralize the label.
    #[serde(rename = "@plural", default)]
    pub plural: LabelPluralize,
    /// Override formatting style.
    #[serde(flatten)]
    pub formatting: Formatting,
    /// Add prefix and suffix.
    #[serde(flatten)]
    pub affixes: Affixes,
    /// Transform the text case.
    #[serde(rename = "text-case")]
    pub text_case: Option<TextCase>,
    /// Remove periods from the output.
    ///
    /// Default: `false`
    #[serde(rename = "strip-periods", default)]
    pub strip_periods: Boolean,
}

/// How to pluralize a label.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelPluralize {
    /// Match plurality of the variable.
    #[default]
    Contextual,
    /// Always use the plural form.
    Always,
    /// Always use the singular form.
    Never,
}

/// A group of formatting instructions that is only shown if no variable is
/// referenced or at least one referenced variable is populated.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Group {
    /// The formatting instructions.
    #[serde(rename = "$value")]
    pub children: Vec<LayoutRenderingElement>,
    // TODO: Roll into proc-macro because #[serde(flatten)] doesn't work with
    // $value fields.
    /// Override formatting style.
    // #[serde(flatten)]
    // pub formatting: Formatting,
    // /// Add prefix and suffix.
    // #[serde(flatten)]
    // pub affixes: Affixes,
    /// Delimit pieces of the output.
    #[serde(rename = "@delimiter")]
    pub delimiter: Option<String>,
    /// Set layout level.
    #[serde(rename = "@display")]
    pub display: Option<Display>,
}

/// A conditional group of formatting instructions.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Choose {
    /// If branch of the conditional group.
    #[serde(rename = "if")]
    pub if_: ChooseBranch,
    /// Other branches of the conditional group. The first matching branch is used.
    #[serde(rename = "else-if")]
    #[serde(default)]
    pub else_if: Vec<ChooseBranch>,
    /// The formatting instructions to use if no branch matches.
    #[serde(rename = "else")]
    pub otherwise: Option<ElseBranch>,
}

/// A single branch of a conditional group.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct ChooseBranch {
    /// Other than this choose, two elements would result in the same
    /// rendering.
    #[serde(rename = "@disambiguate")]
    pub disambiguate: Option<Boolean>,
    /// The variable contains numeric data.
    #[serde(rename = "@is-numeric")]
    /// The variable contains an approximate date.
    pub is_numeric: Option<Vec<Variable>>,
    /// The variable contains an approximate date.
    #[serde(rename = "@is-uncertain-date")]
    pub is_uncertain_date: Option<Vec<DateVariable>>,
    /// The locator matches the given type.
    #[serde(rename = "@locator")]
    pub locator: Option<Vec<Locator>>,
    /// Tests the position of this citation in the citations to the same item.
    /// Only ever true for citations.
    #[serde(rename = "@position")]
    pub position: Option<Vec<TestPosition>>,
    /// Tests whether the item is of a certain type.
    #[serde(rename = "@type")]
    pub type_: Option<Vec<Kind>>,
    #[serde(rename = "@variable")]
    /// Tests whether the default form of this variable is non-empty.
    pub variable: Option<Vec<Variable>>,
    /// How to handle the set of tests.
    #[serde(rename = "@match")]
    #[serde(default)]
    pub match_: ChooseMatch,
    #[serde(rename = "$value", default)]
    /// The formatting instructions to use if the condition matches.
    pub children: Vec<LayoutRenderingElement>,
}

impl ChooseBranch {
    /// Retrieve the test of this branch. Valid CSL files must return `Some`
    /// here.
    pub fn test(&self) -> Option<ChooseTest> {
        if let Some(disambiguate) = self.disambiguate {
            if !*disambiguate {
                None
            } else {
                Some(ChooseTest::Disambiguate)
            }
        } else if let Some(is_numeric) = &self.is_numeric {
            Some(ChooseTest::IsNumeric(is_numeric))
        } else if let Some(is_uncertain_date) = &self.is_uncertain_date {
            Some(ChooseTest::IsUncertainDate(is_uncertain_date))
        } else if let Some(locator) = &self.locator {
            Some(ChooseTest::Locator(locator))
        } else if let Some(position) = &self.position {
            Some(ChooseTest::Position(position))
        } else if let Some(type_) = &self.type_ {
            Some(ChooseTest::Type(type_))
        } else {
            self.variable.as_ref().map(|variable| ChooseTest::Variable(variable))
        }
    }
}

/// The formatting instructions to use if no branch matches.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct ElseBranch {
    /// The formatting instructions.
    /// TODO: May need to accept <cs:layout>.
    #[serde(rename = "$value")]
    children: Vec<LayoutRenderingElement>,
}

/// A single test in a conditional group.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ChooseTest<'a> {
    /// Other than this choose, two elements would result in the same
    /// rendering.
    Disambiguate,
    /// The variable contains numeric data.
    IsNumeric(&'a [Variable]),
    /// The variable contains an approximate date.
    IsUncertainDate(&'a [DateVariable]),
    /// The locator matches the given type.
    Locator(&'a [Locator]),
    /// Tests the position of this citation in the citations to the same item.
    /// Only ever true for citations.
    Position(&'a [TestPosition]),
    /// Tests whether the item is of a certain type.
    Type(&'a [Kind]),
    /// Tests whether the default form of this variable is non-empty.
    Variable(&'a [Variable]),
}

/// Possible positions of a citation in the citations to the same item.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TestPosition {
    /// The first citation to the item.
    First,
    /// Previously cited.
    Subsequent,
    /// Directly following a citation to the same item but the locators don't necessarily match.
    IbidWithLocator,
    /// Directly following a citation to the same item with the same locators.
    Ibid,
    /// Other citation within `near-note-distance` of the same item.
    NearNote,
}

/// How to handle the set of tests in a conditional group.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChooseMatch {
    /// All tests must match.
    #[default]
    All,
    /// At least one test must match.
    Any,
    /// No test must match.
    None,
}

/// A reusable set of formatting instructions.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct CslMacro {
    /// The name of the macro.
    #[serde(rename = "@name")]
    pub name: String,
    // /// The formatting instructions.
    // #[serde(rename = "$value")]
    // pub children: Vec<LayoutRenderingElement>,
}

/// Root element of a locale file.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LocaleRoot {
    /// The version of the locale file.
    pub version: String,
    /// Which languages or dialects this data applies to.
    pub lang: LocaleCode,
    /// Metadata of the locale.
    pub locale_info: Option<LocaleInfo>,
    /// The terms used in the locale.
    pub terms: Terms,
    /// How to format dates in the locale.
    /// file.
    pub date: DateLocale,
    /// Style options for the locale.
    pub style_options: LocaleOptions,
}

/// Supplemental localization data in a citation style.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct InlineLocale {
    /// Which languages or dialects this data applies to. Must be `Some` if this
    /// appears in a locale file.
    #[serde(rename = "@xml:lang")]
    pub lang: Option<LocaleCode>,
    /// Metadata of the locale.
    #[serde(rename = "info")]
    pub locale_info: Option<LocaleInfo>,
    /// The terms used in the locale.
    pub terms: Option<Terms>,
    /// How to format dates in the locale file.
    #[serde(default)]
    pub date: Vec<DateLocale>,
    /// Style options for the locale.
    pub style_options: Option<LocaleOptions>,
}

/// Metadata of a locale.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct LocaleInfo {
    /// The translators of the locale.
    #[serde(rename = "translator")]
    pub translators: Vec<StyleAttribution>,
    /// The license under which the locale is published.
    pub rights: Option<License>,
    /// When the locale was last updated.
    pub updated: Option<Timestamp>,
}

/// Term localization container.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Terms {
    /// The terms.
    #[serde(rename = "term")]
    pub terms: Vec<LocalizedTerm>,
}

/// A localized term.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct LocalizedTerm {
    /// The term key.
    #[serde(rename = "@name")]
    pub name: Term,
    /// The localization.
    #[serde(rename = "$text")]
    localization: Option<String>,
    /// The singular variant.
    single: Option<String>,
    /// The plural variant.
    multiple: Option<String>,
    /// The variant of this term translation.
    #[serde(rename = "@form", default)]
    pub form: TermForm,
    /// Specify the when this ordinal term is used.
    #[serde(rename = "@match")]
    pub match_: Option<OrdinalMatch>,
    /// Specify for which grammatical gender this term has to get corresponding ordinals
    #[serde(rename = "@gender")]
    pub gender: Option<GrammarGender>,
    /// Specify which grammatical gender this ordinal term matches
    #[serde(rename = "@gender-form")]
    pub gender_form: Option<GrammarGender>,
}

impl LocalizedTerm {
    /// Get the singular variant of this term translation. Shall be defined for
    /// valid CSL files.
    pub fn single(&self) -> Option<&str> {
        self.single.as_deref().and(self.localization.as_deref())
    }

    /// Get the plural variant of this term translation. Shall be defined for
    /// valid CSL files.
    pub fn multiple(&self) -> Option<&str> {
        self.multiple.as_deref().and(self.localization.as_deref())
    }
}

/// The variant of a term translation.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TermForm {
    /// The default variant.
    #[default]
    Long,
    /// The short noun variant.
    Short,
    /// The related verb.
    Verb,
    /// The related verb (short form).
    VerbShort,
    /// The symbol variant.
    Symbol,
}

impl TermForm {
    /// Which form is the next fallback if this form is not available.
    pub const fn fallback(self) -> Self {
        match self {
            Self::Long => Self::Long,
            Self::Short => Self::Long,
            Self::Verb => Self::Long,
            Self::VerbShort => Self::Verb,
            Self::Symbol => Self::Short,
        }
    }
}

/// Specify when which ordinal term is used.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrdinalMatch {
    /// Match the last digit for ordinal terms between zero and nine and the
    /// last two otherwise.
    #[default]
    LastDigit,
    /// Always match on the last two non-zero digits.
    LastTwoDigits,
    /// Match on the exact number.
    WholeNumber,
}

/// A grammatical gender. Use `None` for neutral.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrammarGender {
    Feminine,
    Masculine,
}

/// Formats a date in a locale.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct DateLocale {
    /// How the localized date should be formatted.
    #[serde(rename = "@form")]
    pub form: Option<DateForm>,
    /// Which parts of the localized date should be included.
    #[serde(rename = "@date-parts")]
    pub parts: Option<DateParts>,
    /// Override the default date parts. Also specifies the order of the parts
    /// if `form` is `None`.
    #[serde(rename = "$value")]
    pub children: Vec<DatePart>,
}

/// Options for the locale.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct LocaleOptions {
    /// Only use ordinals for the first day in a month.
    ///
    /// Default: `false`
    #[serde(rename = "@limit-day-ordinals-to-day-1")]
    pub limit_day_ordinals_to_day_1: Option<Boolean>,
    /// Whether to place punctuation inside of quotation marks.
    ///
    /// Default: `false`
    #[serde(rename = "@punctuation-in-quote")]
    pub punctuation_in_quote: Option<Boolean>,
}

/// Formatting properties.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Formatting {
    /// Set the font style.
    #[serde(rename = "@font-style")]
    pub font_style: Option<FontStyle>,
    /// Choose normal or small caps.
    #[serde(rename = "@font-variant")]
    pub font_variant: Option<FontVariant>,
    /// Set the font weight.
    #[serde(rename = "@font-weight")]
    pub font_weight: Option<FontWeight>,
    /// Choose underlining.
    #[serde(rename = "@text-decoration")]
    pub text_decoration: Option<TextDecoration>,
    /// Choose vertical alignment.
    #[serde(rename = "@vertical-align")]
    pub vertical_align: Option<VerticalAlign>,
}

/// Font style.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    /// Normal font style.
    #[default]
    Normal,
    /// Italic font style.
    Italic,
}

/// Font variant.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontVariant {
    /// Normal font variant.
    #[default]
    Normal,
    /// Small caps font variant.
    SmallCaps,
}

/// Font weight.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    /// Normal font weight.
    #[default]
    Normal,
    /// Bold font weight.
    Bold,
    /// Light font weight.
    Light,
}

/// Text decoration.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecoration {
    /// No text decoration.
    #[default]
    None,
    /// Underline text decoration.
    Underline,
}

/// Vertical alignment.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    /// No vertical alignment.
    #[default]
    #[serde(rename = "")]
    None,
    /// Align on the baseline.
    Baseline,
    /// Superscript vertical alignment.
    Sup,
    /// Subscript vertical alignment.
    Sub,
}

/// Prefixes and suffixes.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Deserialize)]
pub struct Affixes {
    /// The prefix.
    #[serde(rename = "@prefix")]
    pub prefix: Option<String>,
    /// The suffix.
    #[serde(rename = "@suffix")]
    pub suffix: Option<String>,
}

/// On which layout level to display the citation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Display {
    /// Block stretching from margin to margin.
    Block,
    /// Put in the left margin.
    LeftMargin,
    /// Align on page after `LeftMargin`.
    RightInline,
    /// `Block` and indented.
    Indent,
}

/// How to format text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextCase {
    /// lowecase.
    Lowercase,
    /// UPPERCASE.
    Uppercase,
    /// Capitalize the first word.
    CapitalizeFirst,
    /// Capitalize All Words.
    CapitalizeAll,
    /// Sentence case. *Deprecated*.
    #[serde(rename = "sentence")]
    SentenceCase,
    /// Title case. Only applies to English.
    #[serde(rename = "title")]
    TitleCase,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    fn folder(csl_files: &'static str) {
        let mut failures = 0;
        let mut tests = 0;

        // Read each `.csl` file in the `tests` directory.
        for entry in fs::read_dir(csl_files).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().unwrap() != "csl" || !entry.file_type().unwrap().is_file()
            {
                continue;
            }

            tests += 1;

            let source = fs::read_to_string(&path).unwrap();
            let style_deserializer = &mut Deserializer::from_str(&source);
            style_deserializer.event_buffer_size(EVENT_BUFFER_SIZE);
            let result: Result<Style, _> =
                serde_path_to_error::deserialize(style_deserializer);
            match result {
                // Ok(_) => println!("✅ {:?} passed", &path),
                Ok(_) => {}
                Err(err) => {
                    println!("❌ {:?} failed: \n\n{:#?}", &path, &err);
                    failures += 1;
                }
            }
        }

        if failures == 0 {
            print!("\n🎉")
        } else {
            print!("\n😢")
        }

        println!(" {} out of {} CSL files parsed successfully", tests - failures, tests);

        if failures > 0 {
            panic!("{} tests failed", failures);
        }
    }

    #[test]
    fn test_independent() {
        folder("../../tests/independent");
    }

    #[test]
    fn test_dependent() {
        folder("../../tests/dependent");
    }
}