import React from "react"
import { SchemaDictionary, SessionSettingsDictionary } from "../../sessionManager"

export function Dictionary(props: {
    schema: SchemaDictionary
    session: SessionSettingsDictionary
    setSession: (session: SessionSettingsDictionary) => void
}): JSX.Element {
    return <>todo dictionary</>
}
