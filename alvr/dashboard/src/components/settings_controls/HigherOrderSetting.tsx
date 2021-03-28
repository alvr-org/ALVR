import React from "react"
import { SchemaHOS, useSession } from "../../sessionManager"

export function HighOrderSetting({ schema }: { schema: SchemaHOS }): JSX.Element {
    const { session_settings } = useSession()

    return <>todo HOS</>
}
