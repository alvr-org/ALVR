import React from "react"
import { SchemaVector, SessionSettingsVector } from "../../sessionManager"

export function Vector(props: {
    schema: SchemaVector
    session: SessionSettingsVector
    setSession: (session: SessionSettingsVector) => void
}): JSX.Element {
    return <>todo vector</>
}
