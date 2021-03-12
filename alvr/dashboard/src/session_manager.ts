import subscribeToEvent from "./event_dispatch"

// Taken from settings-schema/src/lib.rs

// Settings representation
type SettingsNode =
    | SettingsSection
    | SettingsChoice
    | null
    | SettingsSwitch
    | boolean
    | number
    | string
    | SettingsNode[]
    | SettingsDictionary

type SettingsSection = {
    [k: string]: SettingsNode
}
type SettingsChoice = {
    type: string
    content: SettingsNode
}
type SettingsOptional = SettingsNode | null
type SettingsSwitch = {
    state: "enabled" | "disabled"
    content?: SettingsNode
}
type SettingsDictionary = [string, SettingsNode][]

// Session settings representation
type SessionSettingsNode =
    | SessionSettingsSection
    | SessionSettingsChoice
    | SessionSettingsOptional
    | SessionSettingsSwitch
    | boolean
    | number
    | string
    | SessionSettingsArray
    | SessionSettingsVector
    | SessionSettingsDictionary

type SessionSettingsSection = {
    [k: string]: SessionSettingsNode
}
type SessionSettingsChoice = {
    variant: string
    [k: string]: SessionSettingsNode | string
}
type SessionSettingsOptional = {
    set: boolean
    content: SessionSettingsNode
}
type SessionSettingsSwitch = {
    enabled: boolean
    content: SessionSettingsNode
}
type SessionSettingsArray = SessionSettingsNode[]
type SessionSettingsVector = {
    element: SessionSettingsNode
    content: SettingsNode[]
}
type SessionSettingsDictionary = {
    key: string
    value: SessionSettingsNode
    content: [string, SettingsNode][]
}

// Schema representation
type SchemaNode =
    | {
          type: "section"
          content: { entries: [string, { advanced: boolean; content: SchemaNode }?][] }
      }
    | {
          type: "choice"
          content: {
              default: string
              variants: [string, { advanced: boolean; content: SchemaNode }?][]
          }
      }
    | {
          type: "optional"
          content: { defaultSet: boolean; content: SchemaNode }
      }
    | {
          type: "switch"
          content: { defaultEnabled: boolean; contentAdvanced: boolean; content: SchemaNode }
      }
    | { type: "boolean"; content: { default: boolean } }
    | {
          type: "integer" | "float"
          content: {
              default: number
              min?: number
              max?: number
              step?: number
              gui?: "textBox" | "upDown" | "slider"
          }
      }
    | {
          type: "text"
          content: { default: string }
      }
    | {
          type: "array"
          content: SchemaNode[]
      }
    | {
          type: "vector"
          content: {
              defaultElement: SchemaNode
              default: SettingsNode[]
          }
      }
    | {
          type: "dictionary"
          content: {
              defaultKey: string
              defaultValue: SchemaNode
              default: [string, SettingsNode][]
          }
      }
