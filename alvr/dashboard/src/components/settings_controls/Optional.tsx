import React from "react"
import { SchemaOptional, SessionSettingsOptional } from "../../sessionManager"

export function OptionalControl(props: {
    schema: SchemaOptional
    session: SessionSettingsOptional
    setSession: (session: SessionSettingsOptional) => void
}): JSX.Element {
    return <>todo optional control</>
}

export function OptionalContainer(props: {
    schema: SchemaOptional
    session: SessionSettingsOptional
    setSession: (session: SessionSettingsOptional) => void
}): JSX.Element | null {
    return <>todo optional content</>
}
