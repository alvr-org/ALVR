import { Col, Row, Space } from "antd"
import React from "react"
import { SchemaNode, SessionSettingsNode } from "../../sessionManager"
import { SettingContainer, SettingControl } from "../Settings"

export function Array(props: {
    schema: SchemaNode[]
    session: SessionSettingsNode[]
    setSession: (session: SessionSettingsNode[]) => void
}): JSX.Element {
    function setContent(index: number, content: SessionSettingsNode) {
        props.session[index] = content

        props.setSession(props.session)
    }

    return (
        <>
            {props.schema.map((schemaContent, index) => {
                const sessionContent = props.session[index]

                return (
                    <Row key={index}>
                        <Col>
                            <Space>
                                <SettingControl
                                    schema={schemaContent}
                                    session={sessionContent}
                                    setSession={c => setContent(index, c)}
                                />
                                <SettingContainer
                                    schema={schemaContent}
                                    session={sessionContent}
                                    setSession={c => setContent(index, c)}
                                />
                            </Space>
                        </Col>
                    </Row>
                )
            })}
        </>
    )
}
