import React from "react"
import { SchemaChoice, SessionSettingsChoice } from "../../sessionManager"

export function Choice(props: {
    schema: SchemaChoice
    session: SessionSettingsChoice
    setSession: (session: SessionSettingsChoice) => void
}): JSX.Element {
    return <>todo choice</>
}
