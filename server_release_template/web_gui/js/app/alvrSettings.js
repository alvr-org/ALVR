
define([
    "json!../../settings-schema",
    "json!../../session",
    "json!../../audio_devices",
    "lib/lodash",
    "i18n!app/nls/settings"


], function (schema, session, audio_devices, _, i18n) {
    return function () {
        var advanced = false;
        var updating = false;

        const video_scales = [25, 50, 66, 75, 100, 125, 150, 200];
        var index = 0;

        this.disableWizard = function () {
            session.setupWizard = false;
            updateSession();
        }

        function init() {

            fillNode(schema, "root", 0, $("#configContent"), "", undefined);
            updateSwitchContent();
            toggleAdvanced();
            addListeners();
            addHelpTooltips();

            setProperties(session.settingsCache, "_root");

            //special case for audio devices
            setDeviceList();
            setVideoScale();

            addChangeListener();

        }

        function setVideoScale() {
            const el = $("#_root_video_resolutionDropdown");
            const targetWidth = $("#_root_video_renderResolution_absolute_width");
            const targetHeight = $("#_root_video_renderResolution_absolute_height");

            const scale = $("#_root_video_renderResolution_scale");

            video_scales.forEach(scale => {
                el.append(`<option value="${scale}"> ${scale}% </option>`)
            });

            el.change((ev) => {
                const val = $(ev.target).val();
                scale.val(val / 100);
                scale.change();
                scale.trigger("input");
            });

        }

        function setDeviceList() {
            const el = $("#_root_audio_gameAudio_content_deviceDropdown");
            const target = $("#_root_audio_gameAudio_content_device");
            let current = "";
            try {
                current = session.settingsCache.audio.gameAudio.content.device;
            } catch (err) {
                console.err("Layout of settings changed, audio devices can not be added. Please report this bug!");
            }
            audio_devices.list.forEach(device => {
                let name = device;
                if (device === audio_devices.default) {
                    name = "(default) " + device;
                }
                el.append(`<option value="${device}"> ${name}  </option>`)
            });

            //set default as current audio device if empty
            if (current === "") {
                target.val(audio_devices.default);
                target.change();
            }

            //move selected audio device to top of list
            var $el = $("#_root_audio_gameAudio_content_deviceDropdown").find("option[value='" + target.val() + "']").remove();
            $("#_root_audio_gameAudio_content_deviceDropdown").find('option:eq(0)').before($el);

            //select the current option in dropdown
            el.val(target.val());

            //add listener to change
            el.change((ev) => {
                target.val($(ev.target).val());
                target.change();
            })
        }

        function getI18n(id) {

            if (i18n === undefined) {
                console.log("names not ready");
                return { "name": id, "description": "" };
            } else {
                if (i18n[id + ".name"] !== undefined) {
                    return { "name": i18n[id + ".name"], "description": i18n[id + ".description"] };;
                } else {
                    console.log("Missing i18n", `"${id}.name":"", \r\n "${id}.description":"", \r\n`);

                    return { "name": id, "description": "" };
                }
            }
        }

        function addChangeListener() {
            $('.parameter input').change((evt) => {
                if (!updating) {
                    var el = $(evt.target);
                    storeParam(el);
                }
            })
        }

        function storeAllParams() {
            $('.parameter input').each((index, el) => {
                storeParam($(el));
            })
        }

        function storeParam(el) {
            var id = el.prop("id");
            var val;

            if (el.prop("type") == "checkbox" || el.prop("type") == "radio") {
                val = el.prop("checked")
            } else {
                if (el.prop("type") == "text" && el.attr("guitype") != "numeric") {
                    val = el.val();
                } else if (el.prop("type") == "radio") {
                    val = el.attr("value");
                } else {
                    val = Number.parseFloat(el.val());
                }
            }
            id = id.replace("_root_", "");
            id = id.replace("-choice-", "");
            var path = id.split("_");


            //choice handling
            if (el.prop("type") == "radio") {
                var name = path[path.length - 1];
                path[path.length - 1] = "variant"
                if (val) {
                    val = name;
                }
            }

            var finalPath = "";
            path.forEach((element, index) => {
                if (Number.isInteger(Number.parseInt(element))) {
                    finalPath += "[" + element + "]";
                } else {
                    if (index != 0) finalPath += ".";
                    finalPath += element;
                }
            });

            _.set(session.settingsCache, finalPath, val);

            updateSession();
        }

        function updateSession() {
            $.ajax({
                type: "POST",
                url: "../../session",
                contentType: "application/json;charset=UTF-8",
                data: JSON.stringify(session),
                processData: false,
                success: function (res) {
                    if (res === "") {
                        console.log("SUCCESS")
                    } else {

                        Lobibox.notify("error", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            title: getI18n("settingsStoreError").name,
                            msg: getI18n("settingsStoreError").description
                        })

                        console.log("FAILED")
                        updating = true;
                        session = res;
                        setProperties(res.settingsCache, "_root");
                        updating = false;
                    }
                },
                error: function (res) {
                    console.log("FAILED")
                    updating = true;
                    session = res;
                    setProperties(res.settingsCache, "_root");
                    updating = false;
                }
            });
        }

        function setProperties(object, path) {

            for (var item in object) {
                if (Array.isArray(object[item])) {
                    object[item].forEach((element, index) => {
                        setProperties(element, path + "_" + item + "_" + index)
                    });
                } else if (Object.prototype.toString.call(object[item]) === '[object Object]') {
                    setProperties(object[item], path + "_" + item);
                } else {


                    var pathItem = item;
                    //choice
                    if (item == "variant") {
                        pathItem = object[item] + "-choice-";
                    }

                    const el = $("#" + path + "_" + pathItem);

                    if (el.length == 0) {
                        console.log("NOT FOUND")
                        console.log("setting value: ", path + "_" + pathItem, object[item])
                    } else {
                        if (el.prop("type") == "checkbox" || el.prop("type") == "radio") {
                            el.prop("checked", object[item])
                        } else {
                            el.val(object[item]);
                        }
                        el.trigger("input");
                        el.change();
                    }
                }
            }
        }

        function updateSwitchContent() {
            $(".switch").each((index, el) => {
                var checked = $(el).find("input").first().prop("checked");
                $(el).find(".card-body input").prop("disabled", !checked)
            })
        }

        function addListeners() {
            $("#toggleAdvanced").click(() => {
                advanced = !advanced;
                toggleAdvanced();
            })

            $(".paramReset").click((evt) => {
                var el = $(evt.target);

                var name = el.attr("name");
                var path = el.attr("path");
                var def = el.attr("default");

                resetToDefault(name, path, def);
            })
        }

        function addHelpTooltips() {
            $('[data-toggle="tooltip"]').tooltip()
        }

        function toggleAdvanced() {
            $("#configContainer .advanced").each((index, el) => {
                if (!advanced) {
                    $(el).addClass("advancedHidden");

                } else {
                    $(el).removeClass("advancedHidden");
                }
            })

            if (advanced) {
                $("#toggleAdvanced i").removeClass("fa-toggle-off");
                $("#toggleAdvanced i").addClass("fa-toggle-on");
            } else {
                $("#toggleAdvanced i").removeClass("fa-toggle-on");
                $("#toggleAdvanced i").addClass("fa-toggle-off");
            }
            //addHelpTooltips();
        }

        //nodes
        function fillNode(node, name, level, element, path, parentType, advanced = false) {
            index += 1;

            if (node == null) {

                switch (name) {
                    case "deviceDropdown":
                    case "resolutionDropdown":
                        addDropdown(element, path, name, advanced)
                        break;
                    default:
                        console.log("null", name);
                        break;
                }
                return;
            }

            switch (node.type) {

                case "section":

                    //section in level 1
                    if (level == 1) {
                        element = createTab(element, path, name, advanced);

                    } else if (level > 1) {

                        if (parentType != "switch") { //switch adds section
                            element = addContainer(element, path, name, advanced);
                        }
                    }

                    var newPath = path + "_" + name;
                    if (parentType == "array") {
                        newPath = path;
                    } else if (parentType == "switch") {
                        newPath = path + "_" + name + "_content";
                    } else if (parentType == "choice") {
                        newPath = path;
                    }

                    node.content.entries.forEach(el => {
                        if (el[1] != null) {
                            fillNode(el[1].content, el[0], level + 1, element, newPath, node.type, el[1].advanced);
                        } else {
                            fillNode(null, el[0], level + 1, element, newPath, node.type);
                        }
                    });
                    break;

                case "switch":

                    if (level == 1) {
                        element = createTab(element, path, name, advanced);
                        element = addSwitchContainer(element, path, name, advanced, node);
                    } else if (level > 1) {
                        element = addSwitchContainer(element, path, name, advanced, node);
                    }

                    fillNode(node.content.content, name, level + 1, element, path, node.type, node.content.advanced);
                    break;

                case "array":
                    element = addContainer(element, path, name, advanced);
                    node.content.forEach((el, index) => {
                        var arrayName = name + "_" + index


                        fillNode(el, arrayName, level + 1, element, path + "_" + arrayName, "array", el.advanced);

                    });

                    break;

                case "choice":

                    element = addRadioContainer(element, path, name, advanced, node);
                    node.content.variants.forEach((el, index) => {

                        var variantElement = addRadioVariant(element, path + "_" + name, el[0], advanced, name, el[1], el[0] == node.content.default);

                        if (el[1] != null) {
                            fillNode(el[1].content, el[0], level + 1, variantElement, path + "_" + name + "_" + el[0], "choice", el[1].advanced);
                        }

                    });

                    break;

                case "integer":
                case "float":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addNumericType(element, path, name, advanced, node);
                    break;

                case "boolean":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addBooleanType(element, path, name, advanced, node);
                    break;

                case "text":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addTextType(element, path, name, advanced, node);
                    break;

                default:
                    element.append(`<div ">
            <h6 class="card-title">
                 ${name}  ${node.type}
            </h6>
            </div>`);
                    console.log("got other type:", name, node.type, path)

            }

        }

        function createTab(element, path, name, advanced) {

            $("#configTabs").append(`
                    <li class="nav-item ${getAdvancedClass(advanced)}">
                        <a class="nav-link" data-toggle="tab" href="#${path + "_" + name}" id="${path + "_" + name + "_tab"}">${getI18n(path + "_" + name + "_tab").name}</a>
                    </li>                    
                    `);
            $("#configContent").append(`
                    <div class="tab-pane fade ${getAdvancedClass(advanced)}" id="${path + "_" + name}" role="tabpanel" >
                    </div>`);

            //check if the tab is the first, then set classes to activate. First child is advanced button
            if ($("#configContent").children().length == 2) {
                $("#" + path + "_" + name).addClass("show active")
                $("#" + path + "_" + name + "_tab").addClass("show active")
            }

            element = $("#" + path + "_" + name);
            return element;
        }

        function addContainer(element, path, name, advanced) {

            var el = `<div class="parameter ${getAdvancedClass(advanced)}">
                <div class="card-title">
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">${getI18n(path + "_" + name).name}</a>
                </div>   
                <div id="collapse_${index}" class="collapse show">
                    <div class="card-body">
                    </div>
                </div> 
            </div>`;

            element.append(el);
            element = element.find(".card-body").last();

            return element;
        }

        function addRadioContainer(element, path, name, advanced, node) {
            var el = `<div class="parameter ${getAdvancedClass(advanced)}" >
                <div class="card-title">
                    ${getI18n(path + "_" + name + "-choice-").name}  ${getHelpReset(name + "_" + node.content.default + "-choice-", path, true)}
                </div>   
                <div>
                    <div class="card-body">
                    </div>
                </div> 
            </div>`;

            element.append(el);
            element = element.find(".card-body").last();
            return element;
        }

        function addDropdown(element, path, name, advanced) {
            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" >     
            <label for="${path}_${name}">${getI18n(path + "_" + name).name} </label> 
           
            <select id="${path}_${name}" >
           
            </select>
        </div>`);


        }

        function addRadioVariant(element, path, name, advanced, radioName, node, isDefault) {
            let checked = "";
            if (isDefault) {
                checked = "checked";
            }

            var el = `<div class="${getAdvancedClass(advanced)}" >
                <input type="radio" id="${path}_${name}-choice-" name="${radioName}"  value="${name}" ${checked}> 
                <label for="${path}_${name}-choice-">${getI18n(path + "_" + name + "-choice-").name}</label>
                <div class="radioContent">
                </div>
            </div>`;

            element.append(el);
            element = element.find(".radioContent").last();
            return element;
        }

        function addSwitchContainer(element, path, name, advanced, node) {
            let checked = "";
            if (node.content.defaultEnabled) {
                checked = "checked";
            }

            var el = `<div class="parameter switch ${getAdvancedClass(advanced)}" >   
                <div class="card-title">
                    <input id="${path}_${name}_enabled" type="checkbox" ${checked} " />
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">
                    ${getI18n(path + "_" + name).name}</a> 
                    ${getHelpReset(name + "_enabled", path, node.content.defaultEnabled)}
                </div>   
                <div id="collapse_${index}" class="collapse show">
                    <div class="card-body">
                    </div>      
                </div> 
            </div>`;

            element.append(el);

            $("#" + path + "_" + name + "_enabled").on("change", updateSwitchContent);

            element = element.find(".card-body").last();

            return element;
        }

        function addTextType(element, path, name, advanced, node) {
            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" >     
                        <label for="${path}_${name}">${getI18n(path + "_" + name).name} </label> 
                        ${getHelpReset(name, path, node.content.default)}
                        <input id="${path}_${name}" type="text" value="${node.content.default}" >
                        </input>
                    </div>`);
        }

        function addBooleanType(element, path, name, advanced, node) {
            let checked = "";
            if (node.content.default) {
                checked = "checked";
            }

            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" > 
                        <input id="${path}_${name}" type="checkbox" ${checked} />
                        <label for="${path}_${name}">${getI18n(path + "_" + name).name} ${getMinMaxLabel(node)} </label>
                         ${getHelpReset(name, path, node.content.default)}                         
                    </div>`);
        }

        function addNumericType(element, path, name, advanced, node) {
            let type = getNumericGuiType(node.content);

            let base = `<div class="parameter ${getAdvancedClass(advanced)}" >
                    <label for="${path}_${name}">${getI18n(path + "_" + name).name} ${getMinMaxLabel(node)}: 
                    </label>`;

            switch (type) {
                case "slider":
                    base += `<div class="rangeValue" id="${path}_${name}_label">[${node.content.default}]</div>${getHelpReset(name, path, node.content.default)}
            <input id="${path}_${name}" type="range" min="${node.content.min}" 
            max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}"  >`;
                    break;

                case "upDown":
                case "updown":
                    base += `<input id="${path}_${name}" type="number" min="${node.content.min}" 
            max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}"> ${getHelpReset(name, path, node.content.default)}`;
                    break;

                case "textbox":
                    base += ` <input id="${path}_${name}"  type="text" min="${node.content.min}" guiType="numeric" 
            max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}" > ${getHelpReset(name, path, node.content.default)}`;
                    break;

                default:
                    console.log("numeric type was: ", type)


            }

            element.append(base + `</div>`);

            $("#" + path + "_" + name).on("input", (el) => {
                $("#" + el.target.id + "_label").text("[" + el.target.value + "]")
            });
        }


        //helper
        function getHelpReset(name, path, defaultVal, postFix = "") {
            return `<div class="helpReset">
                <i class="fa fa-question-circle fa-lg helpIcon" data-toggle="tooltip" title="${getHelp(name, path)}" ></i>
                <i class="fa fa-redo fa-lg paramReset" name="${name}${postFix}" path="${path}" default="${defaultVal}")" ></i>
            </div>`;
        }

        function getHelp(name, path, defaultVal) {
            return getI18n(path + "_" + name).description;
        }

        function getAdvancedClass(advanced) {
            var advancedClass = ""
            if (advanced) {
                advancedClass = "advanced";
            }
            return advancedClass;
        }

        function resetToDefault(name, path, defaultVal) {
            if ($("#" + path + "_" + name).prop("disabled")) {
                return;
            }

            console.log("reset", path, name, $("#" + path + "_" + name).prop("type"))

            if ($("#" + path + "_" + name).prop("type") == "checkbox" || $("#" + path + "_" + name).prop("type") == "radio") {
                if (defaultVal == "true") {
                    $("#" + path + "_" + name).prop('checked', true);
                } else {
                    $("#" + path + "_" + name).prop('checked', false);
                }
            } else {
                $("#" + path + "_" + name).val(defaultVal).trigger("input");
            }
            $("#" + path + "_" + name).change();
        }

        function getMinMaxLabel(node) {
            if (node.content.min == null || node.content.max == null) {
                return "";
            } else {
                return `(${node.content.min}-${node.content.max})`
            }
        }

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

        init();
    }
});


