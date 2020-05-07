'use strict'

function getNumericGuiType(nodeContent) {
    let guiType = nodeContent.gui
    if (guiType == null) {
        if (nodeContent.min != null && nodeContent.max != null) {
            if (nodeContent.step != null) {
                guiType = 'slider'
            } else {
                guiType = 'updown'
            }
        } else {
            guiType = 'textbox'
        }
    }
    return guiType
}


function entryBaseHtml(id, contentHtml, strings) {
    const { name, description } = strings[id]

    return `
        <div class="entry">
            ${name}
            ${contentHtml}
            <button class="help" />
            <div class="description" hidden> ${description} </div>
        </div>
    `
}

function createSettingsHtml(node, placeholderHtmlCBs, strings, advanced) {
    const { type: nodeType, content: nodeContent } = node
    switch (nodeType) {
        case 'section':
            let entriesHtml = '<button class="chevron" />'

            for (const [id, maybeEntryData] of nodeContent.entries) {
                if (maybeEntryData != null) {
                    if (!maybeEntryData.advanced || advanced) {
                        const contentHtml = createSettingsHtml(
                            maybeEntryData.content,
                            placeholderHtmlCBs,
                            strings,
                            advanced
                        )
                        entriesHtml += entryBaseHtml(id, contentHtml, strings)
                    }
                } else {
                    entriesHtml += entryBaseHtml(id, placeholderHtmlCBs[id], strings)
                }
            }

            return entriesHtml

        case 'choice':
            let variantsHtml = ''

            // todo use dropdown for >3 variants
            for (const [id, maybeEntryData] of nodeContent.variants) {
                const name = strings[id].name
                variantsHtml += `
                    <label> ${name} </label>
                    <input type="radio"/>
                `
            }

            return variantsHtml + '<button class="revert" />'
        // variant data is unused

        case 'option':
            // unused
            return ''

        case 'switch':
            let contentHtml = ''
            if (nodeContent.contentAdvanced || advanced) {
                contentHtml = createSettingsHtml(
                    nodeContent.content,
                    placeholderHtmlCBs,
                    strings,
                    advanced
                )
            }
            return `
                <input type="checkbox" />
                <button class="revert" />
                ${contentHtml}
            `

        case 'boolean':
            return `
                <input type="checkbox" />
                <button class="revert" />
            `

        case 'integer':
        case 'float':
            const minAttr = nodeContent.min ? `min="${nodeContent.min}"` : ''
            const maxAttr = nodeContent.max ? `max="${nodeContent.max}"` : ''
            const stepAttr = nodeContent.step ? `step="${nodeContent.step}"` : ''
            const guiType = getNumericGuiType(nodeContent)

            return `
                <input type="number" ${minAttr} ${maxAttr} ${stepAttr} class="${guiType}" />
            `

        case 'text':
            return `
                <input type="text" />
                <button class="revert" />
            `
        case 'array':
            const arrayHtml = ''
            for (const element of nodeContent) {
                arrayHtml += createSettingsHtml(
                    element,
                    placeholderHtmlCBs,
                    strings,
                    advanced
                )
            }

            return arrayHtml

        case 'vector':
            // unused
            return ''

        case 'dictionary':
            // unused
            return ''
    }
}