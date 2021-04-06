import React, { CSSProperties, useState } from "react"
import { ConfigProvider, Drawer, Grid, Layout, Menu, Modal, PageHeader, Row, Select } from "antd"
import {
    ApiOutlined,
    AppstoreAddOutlined,
    GlobalOutlined,
    HddOutlined,
    InfoCircleOutlined,
    LineChartOutlined,
    MenuOutlined,
    SettingOutlined,
    TableOutlined,
} from "@ant-design/icons"
import {
    applySessionSettings,
    SessionSettingsChoice,
    SessionSettingsSection,
    SettingsSchema,
    useSession,
} from "./sessionManager"
import { useAsync } from "react-async-hook"
import { useTranslation } from "react-i18next"
import { Trans, TransName } from "./translation"
import { Connect } from "./components/Connect"
import { Statistics } from "./components/Statistics"
import { Presets } from "./components/Presets"
import { Settings } from "./components/Settings"
import { Installation } from "./components/Installation"
import { Logs } from "./components/Logs"
import { About } from "./components/About"

// Import light theme by default to avoid reflow during loading
import "antd/dist/antd.less"

const INITIAL_SELECTED_TAB = "connect"

function MenuEntries({
    isMobile,
    onClick,
}: {
    isMobile: boolean
    onClick: (selected: string) => void
}): JSX.Element {
    const theme = isMobile ? "light" : "dark"
    const style: CSSProperties = !isMobile ? { position: "absolute", bottom: 0 } : {}

    function handleMenuEntryClick({ key }: { key: React.Key }) {
        onClick(key as string)
    }

    return (
        <>
            <Menu
                theme={theme}
                defaultSelectedKeys={[INITIAL_SELECTED_TAB]}
                onClick={handleMenuEntryClick}
            >
                <Menu.Item key="connect" icon={<ApiOutlined />}>
                    <TransName subkey="connect" />
                </Menu.Item>
                <Menu.Item key="statistics" icon={<LineChartOutlined />}>
                    <TransName subkey="statistics" />
                </Menu.Item>
                <Menu.Item key="presets" icon={<AppstoreAddOutlined rotate={-90} />}>
                    <TransName subkey="presets" />
                </Menu.Item>
                <Menu.Item key="settings" icon={<SettingOutlined />}>
                    <TransName subkey="settings" />
                </Menu.Item>
                <Menu.Item key="installation" icon={<HddOutlined />}>
                    <TransName subkey="installation" />
                </Menu.Item>
                <Menu.Item key="logs" icon={<TableOutlined />}>
                    <TransName subkey="logs" />
                </Menu.Item>
                <Menu.Item key="about" icon={<InfoCircleOutlined />}>
                    <TransName subkey="about" />
                </Menu.Item>
            </Menu>
            <Menu theme={theme} selectable={false} style={style} onClick={handleMenuEntryClick}>
                <Menu.Item key="language" icon={<GlobalOutlined />}>
                    <TransName subkey="language" />
                </Menu.Item>
            </Menu>
        </>
    )
}

function DesktopMenu(props: { selectionHandler: (selection: string) => void }): JSX.Element {
    return (
        <Layout.Sider collapsed>
            <MenuEntries isMobile={false} onClick={props.selectionHandler} />
        </Layout.Sider>
    )
}

function MobileMenu(props: { selectionHandler: (selection: string) => void }): JSX.Element {
    const [drawerOpen, setDrawerOpen] = useState(false)
    const [title, setTitle] = useState(INITIAL_SELECTED_TAB)

    function handleMenuEntryClick(selection: string) {
        props.selectionHandler(selection)
        if (selection !== "language") {
            setTitle(selection)
            setDrawerOpen(false)
        }
    }

    return (
        <PageHeader
            backIcon={<MenuOutlined />}
            title={<TransName subkey={title} />}
            onBack={() => setDrawerOpen(true)}
        >
            <Drawer
                visible={drawerOpen}
                closable={false}
                placement="left"
                onClose={() => setDrawerOpen(false)}
            >
                <MenuEntries isMobile onClick={handleMenuEntryClick} />
            </Drawer>
        </PageHeader>
    )
}

export function Dashboard({ settingsSchema }: { settingsSchema: SettingsSchema }): JSX.Element {
    const session = useSession()

    const { t } = useTranslation()

    const theme = ((session.session_settings["extra"] as SessionSettingsSection)[
        "theme"
    ] as SessionSettingsChoice).variant

    if (
        (theme === "SystemDefault" && window.matchMedia("(prefers-color-scheme: dark)").matches) ||
        theme === "Dark"
    ) {
        import("antd/dist/antd.dark.less")
    } else if (theme === "Compact") {
        import("antd/dist/antd.compact.less")
    } else {
        // Already imported on top
    }

    const extraSettings = session.session_settings["extra"] as SessionSettingsSection

    const directionString = (extraSettings["layout_direction"] as SessionSettingsChoice).variant
    const direction = directionString === "LeftToRight" ? "ltr" : "rtl"

    const componentSizeString = (extraSettings["layout_density"] as SessionSettingsChoice).variant
    const componentSize = componentSizeString.toLowerCase() as "small" | "middle" | "large"

    const { xs } = Grid.useBreakpoint()

    const [selectedTab, setSelectedTab] = useState(INITIAL_SELECTED_TAB)

    const futureLanguagesList = useAsync(async () => {
        return (await (await fetch("/languages/list.json")).json()) as Record<string, string>
    }, [])

    let language = extraSettings["language"] as string

    function changeLanguage(modalCloseHandle: () => void) {
        applySessionSettings({ extra: { language } })

        modalCloseHandle()
    }

    function selectionHandler(selection: string) {
        if (selection === "language") {
            Modal.confirm({
                icon: null,
                title: t("language.prompt"),
                width: 250,
                onOk: changeLanguage,
                maskClosable: true,
                okText: t("common.ok"),
                cancelText: t("common.cancel"),
                content: (
                    <Row justify="center">
                        <Select defaultValue={language} onChange={value => (language = value)}>
                            <Select.Option value="">System</Select.Option>
                            {futureLanguagesList.result &&
                                Object.entries(futureLanguagesList.result).map(
                                    ([code, displayName]) => (
                                        <Select.Option key={code} value={code}>
                                            {displayName}
                                        </Select.Option>
                                    ),
                                )}
                        </Select>
                    </Row>
                ),
            })
        } else {
            setSelectedTab(selection)
        }
    }

    return (
        <ConfigProvider {...{ direction, componentSize }}>
            <Layout>
                {xs ? (
                    <MobileMenu selectionHandler={selectionHandler} />
                ) : (
                    <DesktopMenu selectionHandler={selectionHandler} />
                )}
                <Layout.Content style={{ height: "100vh", overflow: "auto" }}>
                    <div hidden={selectedTab != "connect"}>
                        <Trans node="connect">
                            <Connect />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "statistics"}>
                        <Trans node="statistics">
                            <Statistics />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "presets"}>
                        <Trans node="presets">
                            <Presets />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "settings"}>
                        <Trans node="settings">
                            <Settings schema={settingsSchema} />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "installation"}>
                        <Trans node="installation">
                            <Installation />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "logs"}>
                        <Trans node="logs">
                            <Logs />
                        </Trans>
                    </div>
                    <div hidden={selectedTab != "about"}>
                        <Trans node="about">
                            <About />
                        </Trans>
                    </div>
                </Layout.Content>
            </Layout>
        </ConfigProvider>
    )
}
