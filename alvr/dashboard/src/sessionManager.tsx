import React, { useContext, useState } from "react"
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
    content: [string, { content: { content: SchemaNode } }][]
}

// Schema representation
export type SchemaNode =
    | { type: "Section"; content: SchemaSection }
    | { type: "Choice"; content: SchemaChoice }
    | { type: "Optional"; content: SchemaOptional }
    | { type: "Switch"; content: SchemaSwitch }
    | { type: "Boolean"; content: SchemaBoolean }
    | { type: "Integer" | "Float"; content: SchemaNumeric }
    | { type: "Text"; content: SchemaText }
    | { type: "Array"; content: SchemaNode[] }
    | { type: "Vector"; content: SchemaVector }
    | { type: "Dictionary"; content: SchemaDictionary }
export type SchemaSection = [
    string,
    (
        | { type: "Data"; content: { advanced: boolean; content: SchemaNode } }
        | { type: "HigherOrder"; content: SchemaHOS }
        | { type: "Placeholder" }
    ),
][]
export interface SchemaChoice {
    default: string
    variants: [string, { advanced: boolean; content: SchemaNode } | null][]
    gui: "Dropdown" | "ButtonGroup" | null
}
export interface SchemaOptional {
    default_set: boolean
    content: SchemaNode
}
export interface SchemaSwitch {
    default_enabled: boolean
    content_advanced: boolean
    content: SchemaNode
}
export interface SchemaBoolean {
    default: boolean
}
export interface SchemaNumeric {
    default: number
    min: number | null
    max: number | null
    step: number | null
    gui: "TextBox" | "UpDown" | "Slider" | null
}
export interface SchemaText {
    default: string
}
export interface SchemaVector {
    default_element: SchemaNode
    default: SessionSettingsNode[]
}
export interface SchemaDictionary {
    default_key: string
    default_value: SchemaNode
    default: [string, SessionSettingsNode][]
}

export interface SchemaHOS {
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

export type PresetGroup = [string, SchemaHOS][]

type SessionListener = (session: Session) => void

let listener: SessionListener = () => {}
subscribeToEvent("sessionUpdated", () => {
    fetchSession().then(listener)
})

async function fetchSession(): Promise<Session> {
    return await (await fetch("/api/session/load")).json()
}

let SessionContext: React.Context<Session>

export async function initializeSessionManager(): Promise<[SettingsSchema, Session]> {
    const schema = (await (await fetch("/api/settings-schema")).json()) as SettingsSchema
    const session = await fetchSession()

    SessionContext = React.createContext(session)

    return [schema, session]
}

export function SessionContextWrapper(props: {
    children: React.ReactNode
    initialSession: Session
}): JSX.Element {
    const [session, setSession] = useState(props.initialSession)

    listener = session => setSession(session)

    return <SessionContext.Provider value={session}>{props.children}</SessionContext.Provider>
}

export function useSession(): Session {
    return useContext(SessionContext)
}

export function applySessionSettings(sessionSettings: SessionSettingsRoot): void {
    fetch("/api/session/store-settings", { method: "POST", body: JSON.stringify(sessionSettings) })
}
