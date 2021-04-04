import { Col, Input, InputNumber, Row, Slider, Space } from "antd"
import React, { useEffect, useState } from "react"
import { SchemaNumeric } from "../../sessionManager"
import { Reset } from "./Reset"

function NumericSlider(props: {
    default: number
    min: number
    max: number
    step: number
    session: number
    apply: (session: number) => void
}): JSX.Element {
    const [localValue, setLocalValue] = useState(props.session)

    useEffect(() => {
        setLocalValue(props.session)
    }, [props])

    return (
        <Slider
            value={localValue}
            onChange={setLocalValue}
            onAfterChange={props.apply}
            min={props.min}
            max={props.max}
            step={props.step}
            marks={{
                [props.min]: `${props.min}`,
                [props.default]: "Default",
                [props.max]: `${props.max}`,
            }}
        />
    )
}

function NumericUpDown(props: {
    min: number | null
    max: number | null
    step: number
    session: number
    apply: (value: number | string | null) => void
}): JSX.Element {
    const [localValue, setLocalValue] = useState(props.session)

    useEffect(() => {
        setLocalValue(props.session)
    }, [props])

    const decimalPlaces = props.step.toString().split(".")[1]?.length || 0

    return (
        <InputNumber
            value={localValue}
            onChange={setLocalValue}
            onStep={props.apply}
            onBlur={e => props.apply(e.target.value)}
            min={props.min || undefined}
            max={props.max || undefined}
            step={props.step}
            precision={decimalPlaces}
        />
    )
}

function NumericTextBox(props: { session: number; apply: (session: string) => void }): JSX.Element {
    const [localValue, setLocalValue] = useState(props.session.toString())

    useEffect(() => {
        setLocalValue(props.session.toString())
    }, [props])

    return (
        <Input
            value={localValue}
            onChange={e => setLocalValue(e.target.value)}
            onBlur={e => props.apply(e.target.value)}
        />
    )
}

export function NumericControl(props: {
    schema: SchemaNumeric
    session: number
    setSession: (session: number) => void
}): JSX.Element | null {
    function apply(maybeValue: number | string | null) {
        if (maybeValue === null) {
            maybeValue = props.session
        } else if (typeof maybeValue === "string") {
            maybeValue = parseFloat(maybeValue)
        }

        props.setSession(maybeValue)
    }

    if (props.schema.gui === "UpDown" && props.schema.step !== null) {
        return (
            <Space>
                <NumericUpDown
                    min={props.schema.min}
                    max={props.schema.max}
                    step={props.schema.step}
                    session={props.session}
                    apply={apply}
                />
                <Reset
                    default={props.schema.default}
                    display={props.schema.default.toString()}
                    reset={() => apply(props.schema.default)}
                />
            </Space>
        )
    } else {
        return null
    }
}

export function NumericContainer(props: {
    schema: SchemaNumeric
    session: number
    setSession: (session: number) => void
}): JSX.Element | null {
    function apply(maybeValue: number | string | null) {
        if (maybeValue === null) {
            maybeValue = props.session
        } else if (typeof maybeValue === "string") {
            maybeValue = parseFloat(maybeValue)
        }

        props.setSession(maybeValue)
    }

    let container: JSX.Element | null = null

    if (
        props.schema.gui === "Slider" &&
        props.schema.min !== null &&
        props.schema.max !== null &&
        props.schema.step !== null
    ) {
        container = (
            <NumericSlider
                default={props.schema.default}
                min={props.schema.min}
                max={props.schema.max}
                step={props.schema.step}
                session={props.session}
                apply={apply}
            />
        )
    } else if (props.schema.gui !== "UpDown" || props.schema.step === null) {
        container = <NumericTextBox session={props.session} apply={apply} />
    }

    if (container) {
        return (
            <Row>
                <Col flex="auto">{container}</Col>
                <Col>
                    <Reset
                        default={props.schema.default}
                        display={props.schema.default.toString()}
                        reset={() => apply(props.schema.default)}
                    />
                </Col>
            </Row>
        )
    } else {
        return null
    }
}
