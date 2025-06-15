use chrono::{self, NaiveDate, NaiveDateTime};
use uuid::{self, Uuid};

#[cfg(feature = "facet")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The root struct representing the inventory of everything.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Inventory {
    pub id: Uuid,
    pub companies: Vec<Company>,
    pub created_at: NaiveDateTime,
    pub metadata: InventoryMetadata,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct InventoryMetadata {
    pub version: String,
    pub region: String,
}

/// A company represented in the inventory.
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Company {
    pub id: Uuid,
    pub name: String,
    pub address: Location,
    pub proprietor: CompanyProprietor,
    pub members: Vec<CompanyMember>,
    pub offices: Vec<Office>,
    pub goods: Vec<Item>,
    pub created_at: NaiveDateTime,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct CompanyProprietor {
    pub person: Person,
    pub ownership_percent: f32,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Office {
    pub id: Uuid,
    pub name: String,
    pub address: Location,
    pub staff: Vec<CompanyMember>,
    pub stock: Vec<OfficeStock>,
    pub open: bool,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct OfficeStock {
    pub item: Item,
    pub quantity: u32,
    pub location_code: Option<String>,
}

/// A member of the company
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct CompanyMember {
    pub person: Person,
    pub roles: Vec<Position>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Person {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile: PersonProfile,
    pub preferences: Preferences,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct PersonProfile {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: Sex,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub home_address: Location,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Location {
    pub street: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub geo: Option<Coordinates>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Sex {
    Male,
    Female,
    Other,
    PreferNotToSay,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Item {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: u64,
    pub currency: String,
    pub available: bool,
    pub metadata: Option<ItemMetadata>,
    pub reviews: Vec<ItemReview>,
    pub categories: Vec<Group>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemMetadata {
    pub sku: Option<String>,
    pub categories: Vec<String>,
    pub weight_grams: Option<u32>,
    pub dimensions: Option<ItemDimensions>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemDimensions {
    pub length_mm: Option<f32>,
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct ItemReview {
    pub id: Uuid,
    pub reviewer: PersonSummary,
    pub rating: u8,
    pub text: Option<String>,
    pub created_at: NaiveDateTime,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent: Option<Box<Group>>,
}

/// Brief person reference (for lists)
#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct PersonSummary {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Position {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Right>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Right {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub struct Preferences {
    pub person_id: Uuid,
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub theme: Style,
    pub language: String,
}

#[cfg_attr(feature = "facet", derive(Facet))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum Style {
    Light,
    Dark,
    System,
}
