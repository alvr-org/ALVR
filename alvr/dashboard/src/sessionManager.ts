// Taken from settings-schema/src/lib.rs

// Session settings representation
type SessionSettingsNode =
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

interface SessionSettingsSection {
    [k: string]: SessionSettingsNode
}
interface SessionSettingsChoice {
    variant: string
    [k: string]: SessionSettingsNode
}
interface SessionSettingsOptional {
    set: boolean
    content: SessionSettingsNode
}
interface SessionSettingsSwitch {
    enabled: boolean
    content: SessionSettingsNode
}
interface SessionSettingsVector {
    element: SessionSettingsNode
    content: SessionSettingsNode[]
}
interface SessionSettingsDictionary {
    key: string
    value: SessionSettingsNode
    content: [string, SessionSettingsNode][]
}

// Schema representation
type SchemaNode =
    | {
          type: "section"
          content: { entries: [string, { advanced: boolean; content: SchemaNode } | null][] }
      }
    | {
          type: "choice"
          content: {
              default: string
              variants: [string, { advanced: boolean; content: SchemaNode } | null][]
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
              min: number | null
              max: number | null
              step: number | null
              gui: "textBox" | "upDown" | "slider" | null
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
              default: SessionSettingsNode[]
          }
      }
    | {
          type: "dictionary"
          content: {
              defaultKey: string
              defaultValue: SchemaNode
              default: [string, SessionSettingsNode][]
          }
      }
