// Taken from settings-schema/src/lib.rs

// Session settings representation
export type SessionSettingsNode =
    | SessionSettingsSection
    | SessionSettingsChoice
    | SessionSettingsOptional
    | SessionSettingsSwitch
    | boolean
    | number
    | string
    | SessionSettingsNode[]
    | SessionSettingsVector
    | SessionSettingsDictionary

export interface SessionSettingsSection {
    [k: string]: SessionSettingsNode
}
export interface SessionSettingsChoice {
    variant: string
    [k: string]: SessionSettingsNode
}
export interface SessionSettingsOptional {
    set: boolean
    content: SessionSettingsNode
}
export interface SessionSettingsSwitch {
    enabled: boolean
    content: SessionSettingsNode
}
export interface SessionSettingsVector {
    element: SessionSettingsNode
    content: SessionSettingsNode[]
}
export interface SessionSettingsDictionary {
    key: string
    value: SessionSettingsNode
    content: [string, SessionSettingsNode][]
}

// Schema representation
export type SchemaNode =
    | {
          type: "Section"
          content: [
              string,
              (
                  | {
                        type: "Data"
                        content: { advanced: boolean; content: SchemaNode }
                    }
                  | {
                        type: "HigherOrder"
                        content: Preset
                    }
                  | {
                        type: "Placeholder"
                    }
              ),
          ][]
      }
    | {
          type: "Choice"
          content: {
              default: string
              variants: [string, { advanced: boolean; content: SchemaNode } | null][]
              gui: "Dropdown" | "ButtonGroup" | null
          }
      }
    | {
          type: "Optional"
          content: { default_set: boolean; content: SchemaNode }
      }
    | {
          type: "Switch"
          content: { default_enabled: boolean; content_advanced: boolean; content: SchemaNode }
      }
    | { type: "Boolean"; content: { default: boolean } }
    | {
          type: "Integer" | "Float"
          content: {
              default: number
              min: number | null
              max: number | null
              step: number | null
              gui: "TextBox" | "UpDown" | "Slider" | null
          }
      }
    | {
          type: "Text"
          content: { default: string }
      }
    | {
          type: "Array"
          content: SchemaNode[]
      }
    | {
          type: "Vector"
          content: {
              default_element: SchemaNode
              default: SessionSettingsNode[]
          }
      }
    | {
          type: "Dictionary"
          content: {
              default_key: string
              default_value: SchemaNode
              default: [string, SessionSettingsNode][]
          }
      }

export interface Preset {
    data_type:
        | {
              type: "Choice"
              content: {
                  default: string
                  variants: string[]
                  gui: "Dropdown" | "ButtonGroup" | null
              }
          }
        | {
              type: "Boolean"
              content: {
                  default: boolean
              }
          }
        | {
              type: "Action"
          }
    modifiers: string[]
}

export type PresetGroup = [string, Preset][]
