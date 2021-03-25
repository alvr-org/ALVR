import React from "react"
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

let listener: SessionListener = () => {}

subscribeToEvent("sessionUpdated", () => {
    fetchSession().then(listener)
})

async function fetchSession(): Promise<Session> {
    return await (await fetch("/api/session/load")).json()
}

export function subscribeToSession(callback: SessionListener): void {
    listener = callback
}

export function applySessionSettings(sessionSettings: SessionSettingsRoot): void {
    fetch("/api/session/store-settings", {
        body: JSON.stringify(sessionSettings),
    })
}

export let settingsSchema: SettingsSchema
export let SessionContext: React.Context<Session>

export async function initializeSessionManager(): Promise<Session> {
    settingsSchema = (await (await fetch("/api/settings-schema")).json()) as SettingsSchema

    const session = await fetchSession()
    SessionContext = React.createContext(session)

    return session
}
