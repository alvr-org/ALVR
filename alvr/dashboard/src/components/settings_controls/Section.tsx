import React, { useContext } from "react"
import { SchemaSection, SessionSettingsNode, SessionSettingsSection } from "../../sessionManager"
import { AdvancedContext, generateSettingsControls } from "../Settings"
import { AudioDropdown } from "./AudioDropdown"
import { HighOrderSetting } from "./HigherOrderSetting"

export function Section(props: {
    schema: SchemaSection
    session: SessionSettingsSection
    setSession: (session: SessionSettingsSection) => void
}): JSX.Element {
    const showAdvanced = useContext(AdvancedContext)

    function setSectionSession(fieldName: string, content: SessionSettingsNode) {
        props.session[fieldName] = content

        props.setSession(props.session)
    }

    return (
        <>
            {props.schema.map(([fieldName, schemaContent]) => {
                let control: JSX.Element | null = null
                switch (schemaContent.type) {
                    case "Data": {
                        if (showAdvanced || !schemaContent.content.advanced) {
                            control = generateSettingsControls(
                                schemaContent.content.content,
                                props.session[fieldName],
                                session => setSectionSession(fieldName, session),
                            )
                        }
                        break
                    }
                    case "HigherOrder": {
                        if (!showAdvanced) {
                            control = <HighOrderSetting schema={schemaContent.content} />
                        }
                        break
                    }
                    case "Placeholder": {
                        if (!showAdvanced) {
                            switch (fieldName) {
                                case "device_dropdown":
                                case "input_device_dropdown":
                                case "output_device_dropdown": {
                                    control = <AudioDropdown name={fieldName} />
                                    break
                                }
                            }
                        }
                        break
                    }
                }

                if (control) {
                    return (
                        <div>
                            {fieldName} {control}
                        </div>
                    )
                }
            })}
        </>
    )
}
