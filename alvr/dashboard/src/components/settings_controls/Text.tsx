import { Col, Input, Row } from "antd"
import React, { useEffect, useState } from "react"
import { SchemaText } from "../../sessionManager"
import { Reset } from "./Reset"

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
        <Row>
            <Col flex="auto">
                <Input
                    value={localValue}
                    onChange={e => setLocalValue(e.target.value)}
                    onBlur={e => props.setSession(e.target.value)}
                />
            </Col>
            <Col>
                <Reset
                    default={props.schema.default}
                    display={props.schema.default}
                    reset={() => props.setSession(props.schema.default)}
                />
            </Col>
        </Row>
    )
}
