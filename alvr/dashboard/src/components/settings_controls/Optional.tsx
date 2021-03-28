import React from "react"
import { SchemaOptional, SessionSettingsOptional } from "../../sessionManager"

export function Optional(props: {
    schema: SchemaOptional
    session: SessionSettingsOptional
    setSession: (session: SessionSettingsOptional) => void
}): JSX.Element {
    return <>todo optional</>
}
