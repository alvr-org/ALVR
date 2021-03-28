import React from "react"
import { SchemaSwitch, SessionSettingsSwitch } from "../../sessionManager"

export function Switch(props: {
    schema: SchemaSwitch
    session: SessionSettingsSwitch
    setSession: (session: SessionSettingsSwitch) => void
}): JSX.Element {
    return <>todo switch</>
}
