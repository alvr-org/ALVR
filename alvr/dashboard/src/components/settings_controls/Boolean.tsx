import React from "react"
import { SchemaBoolean } from "../../sessionManager"
import { Space, Switch } from "antd"
import { Reset } from "./Reset"
import { useTranslation } from "react-i18next"

export function Boolean(props: {
    schema: SchemaBoolean
    session: boolean
    setSession: (session: boolean) => void
}): JSX.Element {
    const { t } = useTranslation()

    return (
        <Space>
            <Switch checked={props.session} onChange={props.setSession} />
            <Reset
                default={props.schema.default}
                display={
                    props.schema.default
                        ? t("settings.common.switch-enabled")
                        : t("settings.common.switch-disabled")
                }
                reset={() => props.setSession(props.schema.default)}
            />
        </Space>
    )
}
