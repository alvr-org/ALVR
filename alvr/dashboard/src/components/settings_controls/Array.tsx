import { Col, Row } from "antd"
import React, { Fragment } from "react"
import { SchemaNode, SessionSettingsNode } from "../../sessionManager"
import { SettingContainer, SettingControl } from "../Settings"

export function Array(props: {
    schema: SchemaNode[]
    session: SessionSettingsNode[]
    setSession: (session: SessionSettingsNode[]) => void
}): JSX.Element {
    function setContent(index: number, content: SessionSettingsNode) {
        // branches at other indices must be imported, because otherwise the server cannot correctly
        // work with the resulting session object
        const newSession = props.session.map((branch, idx) => (idx === index ? content : branch))
        props.setSession(newSession)
    }

    return (
        <>
            {props.schema.map((schemaContent, index) => {
                const sessionContent = props.session[index]

                const control = (
                    <SettingControl
                        schema={schemaContent}
                        session={sessionContent}
                        setSession={c => setContent(index, c)}
                    />
                )

                const container = (
                    <SettingContainer
                        schema={schemaContent}
                        session={sessionContent}
                        setSession={c => setContent(index, c)}
                    />
                )

                return (
                    <Fragment key={index}>
                        {control && (
                            <Row>
                                <Col>{control}</Col>
                            </Row>
                        )}
                        {control && <Row style={{ height: 8 }} />}
                        {container && (
                            <Row>
                                {!control && <Col flex="32px" />}
                                <Col flex="auto">{container}</Col>
                            </Row>
                        )}
                        {container && <Row style={{ height: 8 }} />}
                    </Fragment>
                )
            })}
        </>
    )
}
