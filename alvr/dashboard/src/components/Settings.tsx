import { Button, Switch, Tabs } from "antd"
import React, { useState } from "react"
import {
    SchemaNode,
    SchemaSection,
    SessionSettingsNode,
    SessionSettingsRoot,
    SessionSettingsSection,
    SettingsSchema,
} from "../sessionManager"
import { Section } from "./settings_controls/Section"

export function Settings(props: {
    schema: SettingsSchema
    session: SessionSettingsRoot
}): JSX.Element {
    const initialTabKey = props.schema.content[0][0]

    const [advanced, setAdvanced] = useState(false)

    return (
        <Tabs
            defaultActiveKey={initialTabKey}
            tabBarExtraContent={
                <div onClick={() => setAdvanced(!advanced)}>
                    <Switch checked={advanced} />
                    <Button type="link">Advanced</Button>
                </div>
            }
        >
            {props.schema.content.map(tabSchema => (
                <Tabs.TabPane tab={tabSchema[0]} key={tabSchema[0]}>
                    {generateSettingsControls(
                        tabSchema[1].content.content,
                        props.session[tabSchema[0]],
                    )}
                </Tabs.TabPane>
            ))}
        </Tabs>
    )
}

function generateSettingsControls(
    schema: SchemaNode,
    session: SessionSettingsNode,
): JSX.Element | null {
    switch (schema.type) {
        case "Section":
            return (
                <Section
                    schema={schema.content as SchemaSection}
                    session={session as SessionSettingsSection}
                />
            )
    }
    return null
}
