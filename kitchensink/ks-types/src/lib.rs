pub use chrono::{self, NaiveDate, NaiveDateTime};
pub use uuid::{self, Uuid};

#[cfg(feature = "facet")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod unused;

/// The root struct representing the catalog of everything.
///
/// Contains a list of all businesses, catalog creation time, and metadata about the catalog.
/// Used as the entry point for the entire data hierarchy.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Catalog {
    /// Catalog unique identifier.
    /// Automatically generated as a UUID to prevent collisions.
    pub id: Uuid,
    /// List of all businesses included in the catalog.
    pub businesses: Vec<Business>,
    /// Timestamp at which this catalog instance was created.
    pub created_at: NaiveDateTime,
    /// Metadata providing additional information about the catalog such as version.
    pub metadata: CatalogMetadata,
}

/// Container for assorted metadata about the catalog itself.
///
/// Includes versioning and geographical information to facilitate deployments and migrations.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct CatalogMetadata {
    /// Semantic version of the catalog data format.
    pub version: String,
    /// Regional code indicating for which area this catalog is valid (e.g. 'us-east').
    pub region: String,
}

/// Represents a single business entity tracked within the catalog.
///
/// Each business owns a collection of branches, users, and products.
/// Useful for multi-tenant systems or organizational tracking.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Business {
    /// Unique business identifier.
    pub id: Uuid,
    /// Display name of the business (e.g. "Joe's Cafe").
    pub name: String,
    /// Official address of business headquarters.
    pub address: Address,
    /// The primary owner info, including ownership percentage.
    pub owner: BusinessOwner,
    /// List of users/employees associated with the business.
    pub users: Vec<BusinessUser>,
    /// List of branch offices/outlets operated by the business.
    pub branches: Vec<Branch>,
    /// The catalog of products sold or managed by the business.
    pub products: Vec<Product>,
    /// Timestamp marking when the business account was created.
    pub created_at: NaiveDateTime,
}

/// Stores information about a business owner, including identity and percentage share.
///
/// Multiple instances can be used for co-ownership scenarios.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct BusinessOwner {
    /// Owner's user profile (can be cross-referenced to the global user list).
    pub user: User,
    /// The fractional percentage of the business this owner holds (out of 100.0).
    pub ownership_percent: f32,
}

/// Represents a single physical or virtual branch/location of a business.
///
/// Each branch may have its own separate staff and inventory.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Branch {
    /// Unique branch identifier.
    pub id: Uuid,
    /// Name of the branch (e.g. "Downtown", "Online").
    pub name: String,
    /// Physical address of the branch, or location details.
    pub address: Address,
    /// List of employees working at this branch.
    pub employees: Vec<BusinessUser>,
    /// Current inventory at this branch location.
    pub inventory: Vec<BranchInventory>,
    /// Tracks whether the branch is open for business or not.
    pub open: bool,
}

/// Models the record of a specific product's inventory at a particular branch.
///
/// Tracks stock counts and optional codes for mapping product locations.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct BranchInventory {
    /// The product represented by this inventory record.
    pub product: Product,
    /// Quantity of the product currently in stock at the branch.
    pub stock: u32,
    /// Optional code for the storage location (such as a shelf, bin, or warehouse location).
    pub location_code: Option<String>,
}

/// A user employed by or otherwise associated with a business.
///
/// Includes assigned roles, current active status, and the join/creation timestamp.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct BusinessUser {
    /// Reference to the global user data for this employee or associate.
    pub user: User,
    /// List of roles (employee types, such as manager, cashier, etc.) assigned to this user.
    pub roles: Vec<Role>,
    /// Whether this user is currently considered active in the business context.
    pub is_active: bool,
    /// Timestamp recording when this user account was created or joined.
    pub created_at: NaiveDateTime,
}

/// Represents an end user or staff member in the system.
///
/// Includes authentication details, profile, preferences, and audit timestamps.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct User {
    /// Globally unique identifier for the user account.
    pub id: Uuid,
    /// The username chosen or assigned for this user (must be unique).
    pub username: String,
    /// The user's email address (used for notifications and login).
    pub email: String,
    /// When this user account was created in the system.
    pub created_at: NaiveDateTime,
    /// Last time the user's profile or account information was updated.
    pub updated_at: NaiveDateTime,
    /// Extended profile information (personal and contact data).
    pub profile: UserProfile,
    /// User account settings and preferences (such as notifications).
    pub settings: Settings,
}

/// Profile of a user, including personal details and biography.
///
/// Can be expanded to support additional traits as needed.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct UserProfile {
    /// Given name of the user.
    pub first_name: String,
    /// Surname or last/family name.
    pub last_name: String,
    /// Date of birth for the user (used for age calculations, eligibility, etc.).
    pub date_of_birth: NaiveDate,
    /// Self-specified gender identity.
    pub gender: Gender,
    /// Optional free-form biography or "about me".
    pub bio: Option<String>,
    /// Optional URL to the user's avatar or profile picture.
    pub avatar_url: Option<String>,
    /// Primary residence or contact address for the user.
    pub home_address: Address,
}

/// A structured representation of a physical or postal address.
///
/// Used extensively for users, businesses, shipping, etc.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Address {
    /// Name/number and street (e.g., "123 Main St").
    pub street: String,
    /// City or town name.
    pub city: String,
    /// State, province, or region.
    pub state: String,
    /// Postal or ZIP code.
    pub postal_code: String,
    /// Country or nation in ISO 3166-1 alpha-2 format.
    pub country: String,
    /// Optional latitude/longitude geolocation data.
    pub geo: Option<GeoLocation>,
}

/// A geographical location using latitude and longitude.
///
/// Used for mapping, delivery, and analytics operations.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct GeoLocation {
    /// Latitude in decimal degrees (WGS84).
    pub latitude: f64,
    /// Longitude in decimal degrees (WGS84).
    pub longitude: f64,
}

/// Enum representing gender identity options for users.
///
/// Can be expanded to include more options, to suit inclusivity requirements.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Gender {
    /// Identifies as male.
    Male,
    /// Identifies as female.
    Female,
    /// Non-binary, genderqueer, or other identities.
    Other,
    /// Chose not to disclose their gender.
    PreferNotToSay,
}

/// Represents a product available through a business or branch.
///
/// Includes pricing, descriptive metadata, categorization, and customer/contributor reviews.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Product {
    /// SKU or catalog-wide unique identifier for the product.
    pub id: Uuid,
    /// Human-readable product name.
    pub name: String,
    /// Optional extended product description, for display or internal notes.
    pub description: Option<String>,
    /// Retail price in the smallest currency unit (e.g., cents).
    pub price_cents: u64,
    /// ISO 4217 currency code (e.g. "USD", "EUR").
    pub currency: String,
    /// Indicates whether the product is currently available for sale/order.
    pub available: bool,
    /// Additional structured product information (SKU, dimensions, etc.).
    pub metadata: Option<ProductMetadata>,
    /// List of reviews left by customers or users for this product.
    pub reviews: Vec<ProductReview>,
    /// Product categories to which this product belongs.
    pub categories: Vec<Category>,
}

/// Holds extra product details like SKU, categories, or physical properties.
///
/// Can be expanded to track additional supply chain or logistic metadata.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ProductMetadata {
    /// Optional Stock Keeping Unit or vendor identification.
    pub sku: Option<String>,
    /// List of category names/IDs this product belongs to, for quick categorization.
    pub categories: Vec<String>,
    /// Optional net weight in grams for this product.
    pub weight_grams: Option<u32>,
    /// Optional specification for product dimensions (for shipping or shelving).
    pub dimensions: Option<ProductDimensions>,
}

/// Details the physical size of a product for logistics and packaging.
///
/// All values are in millimeters for standardization.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ProductDimensions {
    /// Length along the longest side (mm).
    pub length_mm: Option<f32>,
    /// Width (mm).
    pub width_mm: Option<f32>,
    /// Height (mm).
    pub height_mm: Option<f32>,
}

/// Represents a user-created review for a product.
///
/// Includes reviewer details, rating, text, and time of submission.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ProductReview {
    /// Unique identifier for the review.
    pub id: Uuid,
    /// Minimal user information for the review author.
    pub reviewer: UserSummary,
    /// Numeric rating, usually in the range 1-5.
    pub rating: u8,
    /// Optional review text or comment body.
    pub text: Option<String>,
    /// Date and time when this review was created.
    pub created_at: NaiveDateTime,
}

/// A classification category for products, supporting hierarchical relationships.
///
/// Used to organize and present products in structured groupings.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Category {
    /// Unique identifier for the category node.
    pub id: Uuid,
    /// Display label for this category.
    pub name: String,
    /// Optional description of the category's contents and purpose.
    pub description: Option<String>,
    /// Optional reference to the parent category, enabling tree structures.
    pub parent: Option<Box<Category>>,
}

/// Brief user reference (for lists)
///
/// Contains only a subset of the full user information for privacy and efficiency.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct UserSummary {
    /// User's UUID.
    pub id: Uuid,
    /// User's public handle/username.
    pub username: String,
    /// Optional URL to the user's avatar image.
    pub avatar_url: Option<String>,
}

/// Defines a role for access control and group membership within a business.
///
/// Determines user permissions and groupings (e.g. "Manager", "Cashier").
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Role {
    /// Unique role identifier.
    pub id: Uuid,
    /// Human-readable name of role (must be unique per business).
    pub name: String,
    /// Optional summary or details of the role's purpose/responsibility.
    pub description: Option<String>,
    /// Permissions attached to this role, used for authorization.
    pub permissions: Vec<Permission>,
}

/// Fine-grained permission that may be assigned to a role.
///
/// Typically used to enforce security and workflow limits.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Permission {
    /// Identifier for the specific permission action.
    pub id: Uuid,
    /// Text label of the permission (e.g. "edit_products").
    pub name: String,
    /// Optional textual details on scope or usage.
    pub description: Option<String>,
}

/// User-specific interface and communication settings.
///
/// Supports customizing notification delivery and user interface.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Settings {
    /// Reference back to the user this settings profile belongs to.
    pub user_id: Uuid,
    /// Controls whether user will receive emails.
    pub email_notifications: bool,
    /// Controls whether user will receive push notifications.
    pub push_notifications: bool,
    /// UI theme preference.
    pub theme: Theme,
    /// ISO 639-1 language code for interface localization (e.g. "en", "fr").
    pub language: String,
}

/// User interface color/appearance option.
///
/// Used for dark mode/light mode or system default conformance.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Theme {
    /// White or light backgrounds, dark text.
    Light,
    /// Black or dark backgrounds, light text.
    Dark,
    /// Follow device or system preference.
    System,
}
