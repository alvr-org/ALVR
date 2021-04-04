import { UndoOutlined } from "@ant-design/icons"
import { Button, Modal } from "antd"
import React from "react"
import { useTranslation } from "react-i18next"

export function Reset<T>(props: { default: T; display: string; reset: () => void }): JSX.Element {
    const { t } = useTranslation()

    function handleReset() {
        Modal.confirm({
            icon: null,
            onOk: props.reset,
            maskClosable: true,
            okText: t("common.ok"),
            cancelText: t("common.cancel"),
            content: (
                <div
                    dangerouslySetInnerHTML={{
                        __html: t("settings.common.resetPrompt", {
                            value: "<strong>" + props.display + "</strong>",
                        }),
                    }}
                />
            ),
        })
    }

    return (
        <Button onClick={handleReset} type="link">
            <UndoOutlined />
        </Button>
    )
}
