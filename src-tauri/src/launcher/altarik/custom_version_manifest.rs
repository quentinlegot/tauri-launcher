//! module for fabric version detail manifest

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CustomVersionManifest {
    #[serde(rename(serialize = "inheritsFrom", deserialize = "inheritsFrom"))]
    pub inherits_from: String,
    #[serde(rename(serialize = "mainClass", deserialize = "mainClass"))]
    pub main_class: String,
    pub libraries: Vec<CustomLibrary>,
    pub arguments: CustomArguments,
    pub id: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub t_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct CustomLibrary {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct CustomArguments {
    jvm: Vec<String>,
    game: Vec<String>,
}