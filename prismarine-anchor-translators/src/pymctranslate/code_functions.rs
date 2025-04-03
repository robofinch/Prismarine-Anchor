use serde::{Serialize, Deserialize};

use serde_json::Value;

use super::MappingParseError;


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CodeFunction {
    #[serde(rename = "banner_pattern_2u")]
    BannerPattern2u,
    #[serde(rename = "banner_pattern_fu")]
    BannerPatternFu,
    #[serde(rename = "bedrock_chest_connection_other_left")]
    BedrockChestConnectionOtherLeft,
    #[serde(rename = "bedrock_chest_connection_other_left_120")]
    BedrockChestConnectionOtherLeftUpdated,
    #[serde(rename = "bedrock_chest_connection_other_right")]
    BedrockChestConnectionOtherRight,
    #[serde(rename = "bedrock_chest_connection_other_right_120")]
    BedrockChestConnectionOtherRightUpdated,
    #[serde(rename = "bedrock_chest_connection_self")]
    BedrockChestConnectionSelf,
    #[serde(rename = "bedrock_chest_connection_self_120")]
    BedrockChestConnectionSelfUpdated,
    #[serde(rename = "bedrock_chest_fu")]
    BedrockChestFu,
    #[serde(rename = "bedrock_cmd_custom_name_2u")]
    BedrockCmdCustomName2u,
    #[serde(rename = "bedrock_cmd_custom_name_fu")]
    BedrockCmdCustomNameFu,
    #[serde(rename = "bedrock_moving_block_pos_2u")]
    BedrockMovingBlockPos2u,
    #[serde(rename = "bedrock_moving_block_pos_fu")]
    BedrockMovingBlockPosFu,
    #[serde(rename = "bedrock_sign_2u")]
    BedrockSign2u,
    #[serde(rename = "bedrock_sign_2u_120")]
    BedrockSign2uUpdated,
    #[serde(rename = "bedrock_sign_fu")]
    BedrockSignFu,
    #[serde(rename = "bedrock_sign_fu_120")]
    BedrockSignFuUpdated,
    #[serde(rename = "bedrock_skull_rotation_2u")]
    BedrockSkullRotation2u,
}

impl From<CodeFunction> for &'static str {
    fn from(function: CodeFunction) -> Self {
        match function {
            CodeFunction::BannerPattern2u => "banner_pattern_2u",
            CodeFunction::BannerPatternFu => "banner_pattern_fu",
            CodeFunction::BedrockChestConnectionOtherLeft
                => "bedrock_chest_connection_other_left",
            CodeFunction::BedrockChestConnectionOtherLeftUpdated
                => "bedrock_chest_connection_other_left_120",
            CodeFunction::BedrockChestConnectionOtherRight
                => "bedrock_chest_connection_other_right",
            CodeFunction::BedrockChestConnectionOtherRightUpdated
                => "bedrock_chest_connection_other_right_120",
            CodeFunction::BedrockChestConnectionSelf
                => "bedrock_chest_connection_self",
            CodeFunction::BedrockChestConnectionSelfUpdated
                => "bedrock_chest_connection_self_120",
            CodeFunction::BedrockChestFu          => "bedrock_chest_fu",
            CodeFunction::BedrockCmdCustomName2u  => "bedrock_cmd_custom_name_2u",
            CodeFunction::BedrockCmdCustomNameFu  => "bedrock_cmd_custom_name_fu",
            CodeFunction::BedrockMovingBlockPos2u => "bedrock_moving_block_pos_2u",
            CodeFunction::BedrockMovingBlockPosFu => "bedrock_moving_block_pos_fu",
            CodeFunction::BedrockSign2u           => "bedrock_sign_2u",
            CodeFunction::BedrockSign2uUpdated    => "bedrock_sign_2u_120",
            CodeFunction::BedrockSignFu           => "bedrock_sign_fu",
            CodeFunction::BedrockSignFuUpdated    => "bedrock_sign_fu_120",
            CodeFunction::BedrockSkullRotation2u  => "bedrock_skull_rotation_2u",
        }
    }
}

impl CodeFunction {
    pub fn input_type(&self) -> CodeFunctionInput {
        match self {
            Self::BannerPattern2u
                | Self::BannerPatternFu
                | Self::BedrockCmdCustomName2u
                | Self::BedrockCmdCustomNameFu
                | Self::BedrockSign2u
                | Self::BedrockSign2uUpdated
                | Self::BedrockSignFu
                | Self::BedrockSignFuUpdated
                | Self::BedrockSkullRotation2u
                => CodeFunctionInput::Nbt,
            Self::BedrockChestConnectionOtherLeft
                | Self::BedrockChestConnectionOtherLeftUpdated
                | Self::BedrockChestConnectionOtherRight
                | Self::BedrockChestConnectionOtherRightUpdated
                | Self::BedrockChestConnectionSelf
                | Self::BedrockChestConnectionSelfUpdated
                => CodeFunctionInput::NbtPropertiesPosition,
            Self::BedrockChestFu
                => CodeFunctionInput::PropertiesPosition,
            Self::BedrockMovingBlockPos2u
                | Self::BedrockMovingBlockPosFu
                => CodeFunctionInput::NbtPosition,
        }
    }

    pub fn output_type(&self) -> CodeFunctionOutput {
        match self {
            Self::BannerPattern2u
                | Self::BannerPatternFu
                | Self::BedrockChestFu
                | Self::BedrockCmdCustomName2u
                | Self::BedrockCmdCustomNameFu
                | Self::BedrockMovingBlockPos2u
                | Self::BedrockMovingBlockPosFu
                | Self::BedrockSign2u
                | Self::BedrockSign2uUpdated
                | Self::BedrockSignFu
                | Self::BedrockSignFuUpdated
                => CodeFunctionOutput::NewNbt,
            Self::BedrockChestConnectionOtherLeft
                | Self::BedrockChestConnectionOtherLeftUpdated
                | Self::BedrockChestConnectionOtherRight
                | Self::BedrockChestConnectionOtherRightUpdated
                | Self::BedrockChestConnectionSelf
                | Self::BedrockChestConnectionSelfUpdated
                | Self::BedrockSkullRotation2u
                => CodeFunctionOutput::NewProperties,
        }
    }
}

impl CodeFunction {
    pub fn parse(value: Value) -> Result<Self, MappingParseError> {

        #[derive(Serialize, Deserialize)]
        struct CodeFunctionJson {
            function: CodeFunction,
            input:    Vec<String>,
            output:   Vec<String>,
        }

        let code_function: CodeFunctionJson = serde_json::from_value(value)?;

        let correct_input  = code_function.function.input_type().to_vec();
        let correct_output = code_function.function.output_type().to_vec();

        if correct_input.len() != code_function.input.len() {
            return Err(MappingParseError::IncorrectInput(code_function.function.into()));
        } else {
            for i in 0..correct_input.len() {
                if correct_input[i] != code_function.input[i] {
                    return Err(MappingParseError::IncorrectInput(code_function.function.into()));
                }
            }
        }

        if correct_output.len() != code_function.output.len() {
            return Err(MappingParseError::IncorrectOutput(code_function.function.into()));
        } else {
            for i in 0..correct_input.len() {
                if correct_output[i] != code_function.output[i] {
                    return Err(MappingParseError::IncorrectOutput(code_function.function.into()));
                }
            }
        }

        Ok(code_function.function)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeFunctionInput {
    Nbt,
    NbtPosition,
    NbtPropertiesPosition,
    PropertiesPosition,
}

impl CodeFunctionInput {
    pub fn to_vec(self) -> Vec<&'static str> {
        match self {
            Self::Nbt                   => vec!["nbt"],
            Self::NbtPosition           => vec!["nbt", "location"],
            Self::NbtPropertiesPosition => vec!["nbt", "properties", "location"],
            Self::PropertiesPosition    => vec!["properties", "location"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeFunctionOutput {
    NewNbt,
    NewProperties,
}

impl CodeFunctionOutput {
    pub fn to_vec(self) -> Vec<&'static str> {
        match self {
            Self::NewNbt        => vec!["new_nbt"],
            Self::NewProperties => vec!["new_properties"],
        }
    }
}

// todo: implement each code function
