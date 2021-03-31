import { Input } from "antd"
import React, { useEffect, useState } from "react"
import { SchemaText } from "../../sessionManager"

export function Text(props: {
    schema: SchemaText
    session: string
    setSession: (session: string) => void
}): JSX.Element {
    const [localValue, setLocalValue] = useState(props.session)

    useEffect(() => {
        setLocalValue(props.session)
    }, [props])

    return (
        <Input
            value={localValue}
            onChange={e => setLocalValue(e.target.value)}
            onBlur={e => props.setSession(e.target.value)}
        />
    )
}
