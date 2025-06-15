use chrono::{self, NaiveDate, NaiveDateTime};
use uuid::{self, Uuid};

#[cfg(feature = "facet")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The root struct representing the inventory of everything in the system.
///
/// This collects all companies, their items, staff, and related metadata
/// necessary for tracking organizational and logistical structure.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Inventory {
    /// Universally unique identifier for this inventory snapshot.
    pub id: Uuid,
    /// List of companies managed within this inventory.
    pub companies: Vec<Company>,
    /// Date and time this inventory was created.
    pub created_at: NaiveDateTime,
    /// Arbitrary metadata about this inventory file.
    pub metadata: InventoryMetadata,
}

/// Miscellaneous metadata associated with the inventory.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct InventoryMetadata {
    /// Version tag or number for the inventory schema/data.
    pub version: String,
    /// Geographical region or ISO code relevant to the inventory.
    pub region: String,
}

/// Represents a single company and its data within the inventory system.
///
/// Tracks company location, leadership, members, office locations, and goods managed.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Company {
    /// The unique company UUID.
    pub id: Uuid,
    /// The company's formal or legal name.
    pub name: String,
    /// Primary postal address for the company headquarters.
    pub address: Location,
    /// The owning entity/person of this company.
    pub proprietor: CompanyProprietor,
    /// List of current and former members of the company.
    pub members: Vec<CompanyMember>,
    /// All physical (or virtual) office branches of the company.
    pub offices: Vec<Office>,
    /// The products or services this company sells.
    pub goods: Vec<Item>,
    /// Timestamp for when the company record was created in the system.
    pub created_at: NaiveDateTime,
}

/// Describes the proprietor (owner/operator) of a company.
///
/// Includes details about the person and percentage ownership held.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct CompanyProprietor {
    /// The person or main contact who owns the company.
    pub person: Person,
    /// Percent share of the company's ownership, from 0.0 to 100.0.
    pub ownership_percent: f32,
}

/// A physical or virtual office location for a company.
///
/// Tracks local stock, location, and staff assigned to each branch.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Office {
    /// Universally unique ID for the office.
    pub id: Uuid,
    /// Display name for the office, like "Berlin Branch" or "HQ".
    pub name: String,
    /// The street address of this office.
    pub address: Location,
    /// All current members of staff working at this office.
    pub staff: Vec<CompanyMember>,
    /// Current inventory held at this location.
    pub stock: Vec<OfficeStock>,
    /// Whether the office is open for business or not.
    pub open: bool,
}

/// Inventory for a specific item at a specific office.
///
/// Useful for warehouse/inventory management and tracking.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct OfficeStock {
    /// The item held in stock at this office.
    pub item: Item,
    /// Number of units of this item at the location.
    pub quantity: u32,
    /// Optional alphanumeric code denoting warehouse shelf, bin, or sublocation.
    pub location_code: Option<String>,
}

/// Associates a person with a company, and their assigned roles.
///
/// Used for department lists, permissions, and HR tracking.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct CompanyMember {
    /// The full person details for the member.
    pub person: Person,
    /// The list of positions or roles assigned to this member.
    pub roles: Vec<Position>,
    /// Whether this person is currently active in the company.
    pub is_active: bool,
    /// Timestamp when this member was added to the company.
    pub created_at: NaiveDateTime,
}

/// Records all personal information, preferences, and contact data for one individual.
///
/// This struct can represent staff, proprietors, or external reviewers.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Person {
    /// The person's universal unique identifier.
    pub id: Uuid,
    /// Chosen username or unique login handle for this person.
    pub username: String,
    /// Email address associated with this individual.
    pub email: String,
    /// Instant when this profile was first created.
    pub created_at: NaiveDateTime,
    /// When any property of this profile was last updated.
    pub updated_at: NaiveDateTime,
    /// Extended personal information.
    pub profile: PersonProfile,
    /// User's preferences and settings.
    pub preferences: Preferences,
}

/// Describes personal attributes, address, and public-facing details for a person.
///
/// Can be used for constructing profiles or summaries of people in the system.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct PersonProfile {
    /// Person's first or given name.
    pub first_name: String,
    /// Person's last or family name.
    pub last_name: String,
    /// Date of birth as an ISO8601 date.
    pub date_of_birth: NaiveDate,
    /// Self-identified gender or sex for the person.
    pub gender: Sex,
    /// Optional text biography or tagline shown on the profile.
    pub bio: Option<String>,
    /// Optional URL to an avatar/profile image.
    pub avatar_url: Option<String>,
    /// The permanent or home address of the person.
    pub home_address: Location,
}

/// Represents a physical or mailing address.
///
/// Useful for both residential and commercial locations.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Location {
    /// The street address or first line, such as "123 Main St".
    pub street: String,
    /// The city, town, or settlement name.
    pub city: String,
    /// The state, province, or territory.
    pub state: String,
    /// Postal or ZIP code as a string.
    pub postal_code: String,
    /// Country (full name or code) for this address.
    pub country: String,
    /// Optional geographic coordinates (latitude, longitude).
    pub geo: Option<Coordinates>,
}

/// Represents a point in geographic coordinate space.
///
/// Used for map, GIS, or routing applications.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Coordinates {
    /// Latitude in decimal degrees (positive = north, negative = south).
    pub latitude: f64,
    /// Longitude in decimal degrees (positive = east, negative = west).
    pub longitude: f64,
}

/// Reported sex or gender for a person.
///
/// Used for statistics, forms, and demographic compliance.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Sex {
    /// Male or man.
    Male,
    /// Female or woman.
    Female,
    /// Other self-description, non-binary, etc.
    Other,
    /// Prefer not to specify.
    PreferNotToSay,
}

/// Describes a single product, good, or item managed by a company.
///
/// Includes details, categorization, pricing, inventory status, and reviews.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Item {
    /// Unique identifier for the item.
    pub id: Uuid,
    /// Human readable name of the product.
    pub name: String,
    /// Optional long description or details about the item.
    pub description: Option<String>,
    /// Item's price, in the smallest denomination of currency (e.g. cents).
    pub price_cents: u64,
    /// ISO 4217 currency code used for pricing (e.g. "USD").
    pub currency: String,
    /// True if the item is currently available for sale.
    pub available: bool,
    /// Additional metadata and technical details about this item.
    pub metadata: Option<ItemMetadata>,
    /// Reviews created for this item by users or staff.
    pub reviews: Vec<ItemReview>,
    /// Groups or categories this item belongs to.
    pub categories: Vec<Group>,
}

/// Additional structured information about an item.
///
/// Includes identifiers, dimensions, and classification.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemMetadata {
    /// Optional SKU (Stock-Keeping Unit) for the item.
    pub sku: Option<String>,
    /// Category names this item falls under ("Electronics", "Apparel", etc.)
    pub categories: Vec<String>,
    /// Weight of the item in grams.
    pub weight_grams: Option<u32>,
    /// Dimensions (length, width, height) of the item.
    pub dimensions: Option<ItemDimensions>,
}

/// Physical size measurements for an item.
///
/// Units are in millimeters.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemDimensions {
    /// Length in millimeters.
    pub length_mm: Option<f32>,
    /// Width in millimeters.
    pub width_mm: Option<f32>,
    /// Height in millimeters.
    pub height_mm: Option<f32>,
}

/// A customer or staff review for an item.
///
/// Allows for both qualitative and quantitative feedback.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemReview {
    /// Review's unique identifier.
    pub id: Uuid,
    /// Summary of the person who wrote the review.
    pub reviewer: PersonSummary,
    /// Numeric rating (typically out of 5).
    pub rating: u8,
    /// Optional written text for the review.
    pub text: Option<String>,
    /// Timestamp when the review was posted.
    pub created_at: NaiveDateTime,
}

/// Category, group, or collection for organizing items.
///
/// Can represent hierarchies (parent groups) as well.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Group {
    /// Unique group identifier.
    pub id: Uuid,
    /// Name of the group or category.
    pub name: String,
    /// Optional group description.
    pub description: Option<String>,
    /// Parent group this group belongs to (if any).
    pub parent: Option<Box<Group>>,
}

/// Minimal information for referencing a person in lists or as a reviewer.
///
/// Used in compact displays where full profile details aren't needed.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct PersonSummary {
    /// Person's unique identifier.
    pub id: Uuid,
    /// Chosen username.
    pub username: String,
    /// Optionally, a direct link to this person's avatar image.
    pub avatar_url: Option<String>,
}

/// A position, title, or role within a company (e.g., Manager, Engineer).
///
/// Contains permission assignments and optional description.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Position {
    /// Unique identifier for the position.
    pub id: Uuid,
    /// Display name for this position.
    pub name: String,
    /// Optional, human-readable description of the position's function.
    pub description: Option<String>,
    /// List of rights or permissions assigned to this position.
    pub permissions: Vec<Right>,
}

/// A specific permission or right that can be assigned to a user's position.
///
/// Used for granular access control and authorization policies.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Right {
    /// Unique identifier for the right or permission.
    pub id: Uuid,
    /// Short name describing the right (e.g., "edit_inventory").
    pub name: String,
    /// Optional long-form description of what this right allows.
    pub description: Option<String>,
}

/// User-level preferences and notification settings.
///
/// Includes UI theming and language.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Preferences {
    /// Person's unique identifier for whom these preferences apply.
    pub person_id: Uuid,
    /// Whether the user wants to receive email notifications.
    pub email_notifications: bool,
    /// Whether the user wants to receive mobile/app push notifications.
    pub push_notifications: bool,
    /// Preferred style (light, dark, system) for the UI.
    pub theme: Style,
    /// Preferred language/locale code (e.g., "en", "fr").
    pub language: String,
}

/// Supported UI color theme styles.
///
/// Can be selected by the user in their preferences.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Style {
    /// Bright, light-based user interface.
    Light,
    /// Dark mode for low-light environments.
    Dark,
    /// Follows the default set by the operating system ("System" theme).
    System,
}
