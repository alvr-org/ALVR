import { subscribeToEvent } from "./eventDispatch"

export interface Session {
    client_connections: [string, { display_name: string; manual_ips: string[]; trusted: boolean }][]
    session_settings: SessionSettingsRoot
}

export interface SessionSettingsRoot {
    [k: string]: SessionSettingsSection
}

// Type definitions translated from settings-schema/src/lib.rs

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

export interface SettingsSchema {
    // These corresponds to the settings tabs
    content: [string, { content: { content: SchemaSection } }][]
}

export interface SchemaSection {
    type: "Section"
    content: [
        string,
        (
            | { type: "Data"; content: { advanced: boolean; content: SchemaNode } }
            | { type: "HigherOrder"; content: Preset }
            | { type: "Placeholder" }
        ),
    ][]
}

// Schema representation
export type SchemaNode =
    | SchemaSection
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
    | {
          type: "Boolean"
          content: { default: boolean }
      }
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
          content: { default_element: SchemaNode; default: SessionSettingsNode[] }
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
        | { type: "Boolean"; content: { default: boolean } }
        | { type: "Action" }

    modifiers: string[]
}

export type PresetGroup = [string, Preset][]

type SessionListener = (session: Session) => void

const CURRENT_WEB_CLIENT_ID = Math.floor(Math.random() * 2 ** 16).toString()
let listener: SessionListener = () => {}
let schema: SettingsSchema | null = null

// Note: the schema never changes, so it gets stored after the first call of settingsSchema().
// The session is never stored, to avoid de-syncs with the server.

subscribeToEvent("sessionUpdated", data => {
    const { webClientId } = data as { webClientId: string | null }
    if (webClientId != CURRENT_WEB_CLIENT_ID) {
        getSession().then(listener)
    }
})

export async function getSession(): Promise<Session> {
    return await (await fetch("/session/load")).json()
}

export function subscribeToSession(callback: SessionListener): void {
    listener = callback
}

export function applySessionSettings(sessionSettings: SessionSettingsRoot): void {
    fetch("/session/store-settings", {
        body: JSON.stringify({ webClientId: CURRENT_WEB_CLIENT_ID, sessionSettings }),
    })
}

export async function settingsSchema(): Promise<SettingsSchema> {
    if (!schema) {
        schema = (await (await fetch("/settings-schema")).json()) as SettingsSchema
    }
    return schema
}
