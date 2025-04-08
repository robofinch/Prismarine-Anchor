use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use prismarine_anchor_nbt::snbt::VerifiedSnbt;
use prismarine_anchor_translation::datatypes::BlockProperty;

use super::{
   block_property_from_str, MappingParseError, MappingParseOptions,
    NamespacedIdentifier, PropertyName, PropertyNameBoxed, PropertyNameStr, Snbt,
};


/// A structure holding the meaningful contents of a Specification JSON file
#[derive(Debug)]
pub struct SpecificationFile {
    properties_and_defaults: HashMap<PropertyNameBoxed, (Vec<BlockProperty>, usize)>,
    snbt: Option<(NamespacedIdentifier, VerifiedSnbt)>,
}

impl SpecificationFile {
    #[inline]
    pub fn property_options(&self, property: PropertyNameStr) -> Option<&Vec<BlockProperty>> {
        Some(&self.properties_and_defaults.get(property)?.0)
    }

    #[inline]
    pub fn property_default(&self, property: PropertyNameStr) -> Option<&BlockProperty> {
        let property = self.properties_and_defaults.get(property)?;
        Some(&property.0[property.1])
    }

    #[inline]
    pub fn snbt(&self) -> Option<&(NamespacedIdentifier, VerifiedSnbt)> {
        self.snbt.as_ref()
    }

    /// Deserialize a Specification JSON file into a more workable Rust version.
    #[inline]
    pub fn from_json(
        json: &str, opts: MappingParseOptions,
    ) -> Result<SpecificationFile, MappingParseError> {
        parse_specification_file(serde_json::from_str(json)?, opts)
    }
}

/// A not-yet-validated specification file
#[derive(Serialize, Deserialize)]
struct SpecificationJson {
    properties: HashMap<PropertyName, Vec<String>>,
    defaults: HashMap<PropertyName, String>,
    nbt_identifier: Option<Vec<String>>,
    snbt: Option<Snbt>,
}

fn parse_specification_file(
    json: SpecificationJson, opts: MappingParseOptions,
) -> Result<SpecificationFile, MappingParseError> {

    let mut properties: HashMap<PropertyNameBoxed, (Vec<BlockProperty>, usize)> = json
        .properties.into_iter().map(|(property, values)| {

            let snbt_values = values.into_iter().map(|value| {
                Ok(block_property_from_str(&value, &property, opts)?)
            }).collect::<Result<Vec<BlockProperty>, MappingParseError>>()?;

            Ok((property.into_boxed_str(), (snbt_values, 0)))
        }).collect::<Result<_,MappingParseError>>()?;

    let mut defaults: HashMap<PropertyName, BlockProperty> = json
        .defaults.into_iter().map(|(property, value)| {
            let property_value = block_property_from_str(&value, &property, opts)?;
            Ok((property, property_value))
        }).collect::<Result<_, MappingParseError>>()?;

    for key in properties.keys() {
        if !defaults.contains_key(&**key) {
            return Err(MappingParseError::MissingDefault(key.to_string()));
        }
    }

    for key in defaults.keys() {
        if !properties.contains_key(&**key) {
            return Err(MappingParseError::ExtraDefault(key.clone()));
        }
    }

    for (property, (values, index)) in properties.iter_mut() {
        let default_value = defaults.remove(&**property)
            .expect("Every property was confirmed to have a default");
        let default_index = values.iter().position(|x| {
            *x == default_value
        });

        match default_index {
            Some(default_index) => *index = default_index,
            None => return Err(MappingParseError::InvalidDefault {
                property: property.to_string(),
                invalid_value: default_value,
            })
        }
    }

    let SpecificationJson { nbt_identifier, snbt, .. } = json;

    let snbt = match (nbt_identifier, snbt) {
        (Some(identifier), Some(snbt)) => {
            if identifier.len() != 2 {
                return Err(MappingParseError::IdentifierLength(identifier.len()));
            }
            let mut identifier = identifier.into_iter();

            let snbt = VerifiedSnbt::new(snbt, opts.snbt_options)
                .map_err(MappingParseError::InvalidSnbt)?;

            // We know we can call next() exactly twice before getting None
            Some((
                NamespacedIdentifier {
                    namespace: identifier.next().unwrap().into_boxed_str(),
                    path:      identifier.next().unwrap().into_boxed_str(),
                },
                snbt
            ))
        }
        (None, None) => None,
        _ => return Err(MappingParseError::SnbtXorIdentifier)
    };

    Ok(SpecificationFile {
        properties_and_defaults: properties,
        snbt,
    })
}
