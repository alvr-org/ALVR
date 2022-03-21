define([
    "json!../../api/settings-schema",
    "json!../../api/session/load",
    "app/customSettings",
    "lib/lodash",
    "i18n!app/nls/settings",
    "i18n!app/nls/revertRestart",
    "text!app/templates/revertConfirm.html",
    "text!app/templates/restartConfirm.html",
], function (
    schema,
    session,
    CustomSettings,
    _,
    i18n,
    revertRestartI18n,
    revertConfirm,
    restartConfirm
) {
    return function () {
        const self = this;
        let advanced = false;
        let updating = false;
        const customSettings = new CustomSettings(self);
        let index = 0;
        const usedi18n = {};

        function randomAlphanumericID() {
            const len = 10;
            const arr = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghilmnopqrstuvwxyz0123456789";
            let ans = "";
            for (let i = len; i > 0; i--) {
                ans += arr[Math.floor(Math.random() * arr.length)];
            }
            return ans;
        }

        const webClientId = randomAlphanumericID();

        self.disableWizard = function () {
            session.setupWizard = false;
            self.storeSession("other");
        };

        function init() {
            fillNode(schema, "root", 0, $("#configContent"), "", undefined);
            updateSwitchContent();
            updateOptionalContent();

            setProperties(session.sessionSettings, "_root");

            toggleAdvanced();
            addChangeListener();

            //special

            customSettings.setCustomSettings();

            addListeners();
            addHelpTooltips();
            printUnusedi18n();
        }

        self.updateSession = function (newSession) {
            updating = true;
            session = newSession;
            setProperties(newSession.sessionSettings, "_root");
            updating = false;
        };

        self.isUpdating = function () {
            return updating;
        };

        self.getSession = function () {
            return session;
        };

        self.getWebClientId = function () {
            return webClientId;
        };

        function printUnusedi18n() {
            for (const key in i18n) {
                if (usedi18n[key] === undefined) console.log("Unused i18n key:", key);
            }
        }

        function getI18n(id) {
            if (i18n === undefined) {
                console.log("names not ready");
                return { name: id, description: "" };
            } else {
                let name;
                if (i18n[id + ".name"] !== undefined) {
                    usedi18n[id + ".name"] = true;
                    name = i18n[id + ".name"];
                } else {
                    name = id.substring(id.lastIndexOf("_") + 1, id.length).replace("-choice-", "");
                }

                return { name: name, description: i18n[id + ".description"] };
            }
        }

        function addChangeListener() {
            $(".parameter input:not(.skipInput)").change((evt) => {
                if (!updating) {
                    const el = $(evt.target);
                    self.storeParam(el);
                }
            });
        }

        self.storeParam = function (el, skipstoreSession = false) {
            let id = el.prop("id");
            let val;

            if (el.prop("type") == "checkbox" || el.prop("type") == "radio") {
                val = el.prop("checked");
            } else {
                if (el.prop("type") == "text" && el.attr("guitype") != "numeric") {
                    val = el.val();
                } else if (el.prop("type") == "radio") {
                    val = el.attr("value");
                } else {
                    const numericType = el.attr("numericType");
                    if (numericType == "float") {
                        val = Number.parseFloat(el.val());
                        val = clampNumeric(el, val);
                        el.val(val); //input number could have been parsed and altered
                    } else if (numericType == "integer") {
                        val = Number.parseInt(el.val());
                        val = clampNumeric(el, val);
                        el.val(val); //input number could have been parsed and altered
                    }
                }
            }
            id = id.replace("_root_", "");
            id = id.replace("-choice-", "");
            const path = id.split("_");

            //choice handling
            if (el.prop("type") == "radio") {
                const name = path[path.length - 1];
                path[path.length - 1] = "variant";
                if (val) {
                    val = name;
                }
            }

            let finalPath = "";
            path.forEach((element, index) => {
                if (Number.isInteger(Number.parseInt(element))) {
                    finalPath += "[" + element + "]";
                } else {
                    if (index != 0) finalPath += ".";
                    finalPath += element;
                }
            });

            _.set(session.sessionSettings, finalPath, val);

            if (!skipstoreSession) {
                self.storeSession("settings");
            }
        };

        self.storeSession = function (updateType) {
            if (updating) {
                return;
            }

            $.ajax({
                type: "POST",
                url: "/api/session/store",
                contentType: "application/json;charset=UTF-8",
                data: JSON.stringify({
                    updateType: updateType,
                    webClientId: webClientId,
                    session: session,
                }),
                processData: false,
                success: function (res) {
                    if (res === "") {
                        console.log("SUCCESS");
                    } else {
                        Lobibox.notify("error", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            title: getI18n("settingsStoreError").name,
                            msg: getI18n("settingsStoreError").description,
                        });

                        console.log("FAILED");
                        updating = true;
                        session = res;
                        setProperties(res.sessionSettings, "_root");
                        updating = false;
                    }
                },
                error: function (res) {
                    console.log("FAILED");
                    updating = true;
                    session = res;
                    setProperties(res.sessionSettings, "_root");
                    updating = false;
                },
            });
        };

        function setProperties(object, path) {
            for (const item in object) {
                if (Array.isArray(object[item])) {
                    object[item].forEach((element, index) => {
                        setProperties(element, path + "_" + item + "_" + index);
                    });
                } else if (Object.prototype.toString.call(object[item]) === "[object Object]") {
                    setProperties(object[item], path + "_" + item);
                } else {
                    let pathItem = item;
                    //choice
                    if (item == "variant") {
                        pathItem = object[item] + "-choice-";
                    }

                    const el = $("#" + path + "_" + pathItem);

                    if (el.length == 0) {
                        console.log("NOT FOUND");
                        console.log("setting value: ", path + "_" + pathItem, object[item]);
                    } else {
                        if (el.prop("type") == "checkbox") {
                            el.prop("checked", object[item]);
                        } else if (el.prop("type") == "radio") {
                            el.prop("checked", object[item]);
                            el.parent().parent().children().filter(".active").removeClass("active");
                            el.parent().addClass("active");
                            $(
                                `#${el
                                    .parent()
                                    .parent()
                                    .parent()
                                    .attr("id")}radioContent .radioContent`
                            ).hide();
                            $(`div.radioContent[for="${el.attr("id")}"]`).show();
                        } else {
                            el.val(object[item]);
                        }
                        el.trigger("input");
                        el.change();
                    }
                }
            }
        }

        function getProperties(object, path, separator = "_") {
            const properties = Array.isArray(path) ? path : path.split(separator);
            return properties.reduce((prev, curr) => prev && prev[curr], object);
        }

        function updateSwitchContent() {
            $(".switch").each((index, el) => {
                const checked = $(el).find("input").first().prop("checked");
                if (checked) {
                    $(el).find(".card-body").show();
                } else {
                    $(el).find(".card-body").hide();
                }
            });
        }

        function updateOptionalContent() {
            $(".optional").each((index, el) => {
                const checked = $(el).find("input[type='checkbox']").first().prop("checked");

                if (checked) {
                    $(el).find(".optionalSet").button("toggle");
                    $(el).find(".card-body").show();
                } else {
                    $(el).find(".optionalUnset").button("toggle");
                    $(el).find(".card-body").hide();
                }
            });
        }

        function addListeners() {
            $("#toggleAdvanced").click(() => {
                advanced = !advanced;
                toggleAdvanced();
            });

            $("#restartSteamVRButton").click(() => {
                restartSteamVR();
            });

            $(".paramReset").click((evt) => {
                const el = $(evt.target);

                const name = el.attr("name");
                const path = el.attr("path");
                const def = el.attr("default");
                const defText = el.attr("defaultText");

                if (!$("#" + path + "_" + name).prop("disabled")) {
                    const confirm = $("#_root_extra_revertConfirmDialog").prop("checked");
                    if (confirm) {
                        showResetConfirmDialog(def, defText).then((res) => {
                            if (res) {
                                resetToDefault(name, path, def);
                            }
                        });
                    } else {
                        resetToDefault(name, path, def);
                    }
                }
            });
        }

        function addHelpTooltips() {
            $("[data-toggle='tooltip']").tooltip();
        }

        function restartSteamVR() {
            const triggerRestart = () => {
                $.get("restart-steamvr", undefined, (res) => {
                    if (res == 0) {
                        Lobibox.notify("success", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.steamVRRestartSuccess,
                        });
                    }
                });
            };

            const confirm = $("#_root_extra_restartConfirmDialog").prop("checked");
            if (confirm) {
                showRestartConfirmDialog().then((res) => {
                    if (res) {
                        triggerRestart();
                    }
                });
            } else {
                triggerRestart();
            }
        }

        function toggleAdvanced() {
            $("#configContainer .advanced").each((index, el) => {
                if (!advanced) {
                    $(el).addClass("advancedHidden");
                } else {
                    $(el).removeClass("advancedHidden");
                }
            });

            //special cases like device dropdown
            $("#configContainer .special").each((index, el) => {
                if (advanced) {
                    $(el).addClass("advancedHidden");
                } else {
                    $(el).removeClass("advancedHidden");
                }
            });

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
                    case "inputDeviceDropdown":
                    case "outputDeviceDropdown":
                    case "resolutionDropdown":
                    case "headsetEmulationMode":
                    case "controllerMode":
                        addDropdown(element, path, name, advanced);
                        break;
                    case "disableThrottling":
                        addBooleanType(element, path, name, advanced, {
                            content: { default: false },
                        });
                        break;
                    case "bufferOffset":
                        addNumericType(element, path, name, advanced, {
                            content: { default: 0, gui: "slider" },
                        });
                        break;
                    case "trackingSpeed":
                        addHidden(element, path, name, advanced);
                        break;
                    case "displayRefreshRate":
                        addHidden(element, path, name, advanced);
                        break;
                    default:
                        console.log(
                            "Unhandled node without content. Should be implemented as special case:",
                            name
                        );
                        break;
                }
                return;
            }

            //special case for optional and switch, values are now named with "_content"
            if (parentType == "optional" || parentType == "switch") {
                name = name + "_content";
            }

            switch (node.type) {
                case "section":
                    //section in level 1
                    if (level == 1) {
                        element = createTab(element, path, name, advanced);
                    } else if (level > 1) {
                        if (parentType != "switch" && parentType != "optional") {
                            //switch and optional add own sections
                            element = addContainer(element, path, name, advanced);
                        }
                    }

                    let newPath = path + "_" + name;
                    if (parentType == "array") {
                        newPath = path;
                    } else if (parentType == "choice") {
                        newPath = path;
                    }

                    node.content.entries.forEach((el) => {
                        if (el[1] != null) {
                            fillNode(
                                el[1].content,
                                el[0],
                                level + 1,
                                element,
                                newPath,
                                node.type,
                                el[1].advanced
                            );
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

                    fillNode(
                        node.content.content,
                        name,
                        level + 1,
                        element,
                        path,
                        node.type,
                        node.content.advanced
                    );
                    break;

                case "array":
                    element = addContainer(element, path, name, advanced);
                    node.content.forEach((el, index) => {
                        const arrayName = name + "_" + index;

                        fillNode(
                            el,
                            arrayName,
                            level + 1,
                            element,
                            path + "_" + arrayName,
                            "array",
                            el.advanced
                        );
                    });

                    break;

                case "choice":
                    element = addRadioContainer(element, path, name, advanced, node);
                    node.content.variants.forEach((el, index) => {
                        const variantElement = addRadioVariant(
                            element,
                            path + "_" + name,
                            el[0],
                            advanced,
                            name,
                            el[1],
                            el[0] == node.content.default
                        );

                        if (el[1] != null) {
                            fillNode(
                                el[1].content,
                                el[0],
                                level + 1,
                                variantElement,
                                path + "_" + name + "_" + el[0],
                                "choice",
                                el[1].advanced
                            );
                        }
                    });

                    break;

                case "optional":
                    if (level == 1) {
                        element = createTab(element, path, name, advanced);
                        element = addOptionalContainer(element, path, name, advanced, node);
                    } else if (level > 1) {
                        element = addOptionalContainer(element, path, name, advanced, node);
                    }

                    fillNode(
                        node.content.content,
                        name,
                        level + 1,
                        element,
                        path,
                        node.type,
                        node.content.advanced
                    );
                    break;

                case "integer":
                case "float":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addNumericType(element, path, name, advanced, node, node);
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
                    console.log("got other type:", name, node.type, path);
                    break;
            }
        }

        function createTab(element, path, name, advanced) {
            $("#configTabs").append(`
                    <li class="nav-item ${getAdvancedClass(advanced)}">
                        <a class="nav-link" data-toggle="tab" href="#${path + "_" + name}" id="${
                path + "_" + name + "_tab"
            }">${getI18n(path + "_" + name + "_tab").name}</a>
                    </li>                    
                    `);
            $("#configContent").append(`
                    <div class="tab-pane fade ${getAdvancedClass(advanced)}" id="${
                path + "_" + name
            }" role="tabpanel" >
                    </div>`);

            //check if the tab is the first, then set classes to activate. First child is advanced button, second the reload steamVr
            if ($("#configContent").children().length == 3) {
                $("#" + path + "_" + name).addClass("show active");
                $("#" + path + "_" + name + "_tab").addClass("show active");
            }

            element = $("#" + path + "_" + name);
            return element;
        }

        function addContainer(element, path, name, advanced) {
            const el = `<div class="parameter ${getAdvancedClass(advanced)}">
                <div class="card-title">
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">${
                getI18n(path + "_" + name).name
            }</a>
                    ${self.getHelpReset(name, path, true)}
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

        function addOptionalContainer(element, path, name, advanced, node) {
            const defaultSet = !getProperties(
                session,
                path.replace("_root", "sessionSettings") + "_" + name + "_set"
            );
            let checked = "";
            let expanded = true;
            let collapse = "show";
            let collapsed = "";

            if (defaultSet) {
                checked = "checked";
                expanded = false;
                collapse = "";
                collapsed = "collapsed";
            }

            const el = `<div class="parameter optional ${getAdvancedClass(advanced)}" >   
                <div class="card-title">
                    <div class="btn-group btn-group-sm" data-toggle="buttons">
                        <label class="btn btn-primary optionalSet"><input class="skipInput" type="radio" name="${path}_${name}" id="${path}_${name}_setRadio" >Set</label>
                        <label class="btn btn-primary optionalUnset"><input class="skipInput" type="radio" name="${path}_${name}" id="${path}_${name}_defaultRadio" >Default</label>
                    </div>
                    <input id="${path}_${name}_set" type="checkbox" ${checked}  style="visibility:hidden" />
                    <a class="accordion-toggle ${collapsed}" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded=${expanded}>
                    ${getI18n(path + "_" + name).name}</a> 
                    ${self.getHelpReset(name + "_set", path, defaultSet)}
                </div>   
                <div id="collapse_${index}" class="collapse ${collapse}">
                    <div class="card-body">
                    </div>      
                </div> 
            </div>`;

            element.append(el);

            const _path = path;
            const _name = name;
            const _index = index;
            // this call need const variable unless you want them overwriten by the next call.
            $(document).ready(() => {
                $("#" + _path + "_" + _name + "_setRadio")
                    .parent()
                    .click(() => {
                        $("#" + _path + "_" + _name + "_set").prop("checked", true);
                        $("#" + _path + "_" + _name + "_set").change();
                        $("#collapse_" + _index).collapse("show");
                    });
                $("#" + _path + "_" + _name + "_defaultRadio")
                    .parent()
                    .click(() => {
                        $("#" + _path + "_" + _name + "_set").prop("checked", false);
                        $("#" + _path + "_" + _name + "_set").change();
                        $("#collapse_" + _index).collapse("hide");
                    });
            });

            $("#" + path + "_" + name + "_set").on("change", updateOptionalContent);

            element = element.find(".card-body").last();

            return element;
        }

        function addRadioContainer(element, path, name, advanced, node) {
            const el = `<div class="parameter ${getAdvancedClass(advanced)}" >
                <div class="card-title">
                    ${getI18n(path + "_" + name + "-choice-").name}  ${self.getHelpReset(
                name,
                path,
                true,
                "_" + node.content.default + "-choice-",
                name + "-choice-",
                getI18n(path + "_" + name + "_" + node.content.default + "-choice-").name
            )}
                </div>   
                <div>
                <form id="${path + "_" + name + "-choice-"}" class="card-body">
                    <div class="btn-group btn-group-toggle" data-toggle="buttons">
                    </div>
                    <div id="${path + "_" + name + "-choice-" + "radioContent"}"></div>
                </form>
                </div> 
            </div>`;

            element.append(el);
            element = element.find(".btn-group").last();
            return element;
        }

        function addDropdown(element, path, name, advanced) {
            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" >     
            <label for="${path}_${name}">${getI18n(path + "_" + name).name} </label> 
           
            <select id="${path}_${name}" >
           
            </select>
        </div>`);
        }

        /**
         * Used as a genetic type to be replaced/filled by custom settings
         *
         * @param {*} element the html element where the created div will be added
         * @param {string} path patth to the setting
         * @param {string} name the name of the parameter represented by the created div
         * @param {boolean} advanced  flag if the settig is an adanced one
         */
        function addHidden(element, path, name, advanced) {
            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" >     
            <label for="${path}_${name}">${getI18n(path + "_" + name).name} </label> 
           
            <input type="hidden" id="${path}_${name}" >           
            </input>
        </div>`);
        }

        function addRadioVariant(element, path, name, advanced, radioName, node, isDefault) {
            let checked = "";
            let active = "";
            if (isDefault) {
                checked = "checked";
                active = "active";
            }

            const el = `<div class="btn btn-primary" ${getAdvancedClass(advanced)}" >
                <input type="radio" id="${path}_${name}-choice-" name="${radioName}"  value="${name}"> 
                <label for="${path}_${name}-choice-" style="margin-bottom:0">${
                getI18n(path + "_" + name + "-choice-").name
            }</label>
                </div>`;
            const content = `<div class="radioContent" for="${path}_${name}-choice-"></div>`;
            element.next().append(content);
            element.append(el);

            element = element.next().find(".radioContent").last();
            if (!isDefault) {
                element.hide();
            }
            return element;
        }

        function addSwitchContainer(element, path, name, advanced, node) {
            let checked = "";
            if (node.content.defaultEnabled) {
                checked = "checked";
            }

            const el = `<div class="parameter switch ${getAdvancedClass(advanced)}" >   
                <div class="card-title">
                    <input id="${path}_${name}_enabled" type="checkbox" ${checked} " />
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">
                    ${getI18n(path + "_" + name).name}</a> 
                    ${self.getHelpReset(name + "_enabled", path, node.content.defaultEnabled)}
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
                        ${self.getHelpReset(name, path, node.content.default)}
                        <input id="${path}_${name}" type="text" value="${node.content.default}" >
                        </input>
                    </div>`);
        }

        function addBooleanType(element, path, name, advanced, node) {
            let checked = "";
            if (node !== undefined && node.content.default) {
                checked = "checked";
            }

            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" > 
                        <input id="${path}_${name}" type="checkbox" ${checked} />
                        <label for="${path}_${name}">${
                getI18n(path + "_" + name).name
            } ${getMinMaxLabel(node)} </label>
                         ${self.getHelpReset(
                             name,
                             path,
                             node.content.default
                         )}                         
                    </div>`);
        }

        function addNumericType(element, path, name, advanced, node) {
            const type = getNumericGuiType(node.content);

            let base = `<div class="parameter ${getAdvancedClass(advanced)}" >
                    <label for="${path}_${name}">${
                getI18n(path + "_" + name).name
            } ${getMinMaxLabel(node)}: 
                    </label>`;

            switch (type) {
                case "slider":
                    base += `<div class="rangeValue" id="${path}_${name}_label">[${
                        node.content.default
                    }]</div>${self.getHelpReset(name, path, node.content.default)}
                    <input numericType="${node.type}" id="${path}_${name}" type="range" min="${
                        node.content.min
                    }" 
                    max="${node.content.max}" value="${node.content.default}"  step="${
                        node.content.step
                    }"  >`;
                    break;

                case "upDown":
                case "updown":
                    const el = `<input numericType="${node.type}" id="${path}_${name}" type="number" min="${node.content.min}" 
                    max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}">`;

                    const grp = `<div class="upDownGrp" ><div class="input-group">
                    <div class="input-group-prepend">
                        <button class="btn btn-primary btn-sm" id="minus-btn"><i class="fa fa-minus"></i></button>
                    </div>
                    ${el}
                    <div class="input-group-append">
                        <button class="btn btn-primary btn-sm" id="plus-btn"><i class="fa fa-plus"></i></button>
                    </div>
                    
                    </div></div>${self.getHelpReset(name, path, node.content.default)}`;

                    base += grp;
                    break;

                case "textbox":
                    base += ` <input numericType="${
                        node.type
                    }" id="${path}_${name}"  type="text" min="${
                        node.content.min
                    }" guiType="numeric" 
                    max="${node.content.max}" value="${node.content.default}"  step="${
                        node.content.step
                    }" > ${self.getHelpReset(name, path, node.content.default)}`;
                    break;

                default:
                    console.log("numeric type was: ", type);
                    break;
            }

            element.append(base + `</div>`);

            $("#" + path + "_" + name).on("input", (el) => {
                $("#" + el.target.id + "_label").text("[" + el.target.value + "]");
            });

            //add spinner functions
            $("#" + path + "_" + name + "[type=number]")
                .prev()
                .on("click", (el) => {
                    let val = new Number($("#" + path + "_" + name).val());
                    let step = 1;
                    if (node.content.step !== null) {
                        step = node.content.step;
                    }

                    val = val - step;

                    if (node.content.min != null && val < node.content.min) {
                        val = node.content.min;
                    }
                    $("#" + path + "_" + name).val(val);
                    $("#" + path + "_" + name).change();
                });

            $("#" + path + "_" + name + "[type=number]")
                .next()
                .on("click", (el) => {
                    let val = new Number($("#" + path + "_" + name).val());

                    let step = 1;
                    if (node.content.step !== null) {
                        step = node.content.step;
                    }

                    val = val + step;

                    if (node.content.max != null && val > node.content.max) {
                        val = node.content.max;
                    }
                    $("#" + path + "_" + name).val(val);
                    $("#" + path + "_" + name).change();
                });
        }

        //helper
        self.getHelpReset = function (name, path, defaultVal, postFix = "", helpName, defaultText) {
            if (helpName == undefined) {
                helpName = name;
            }

            if (defaultText == undefined) {
                defaultText = defaultVal;
            }

            const getVisibility = function () {
                if (getHelp(helpName, path) === undefined) {
                    return `style="display:none"`;
                }
            };
            return `<div class="helpReset">
                <i class="fa fa-question-circle fa-lg helpIcon" data-toggle="tooltip" title="${getHelp(
                    helpName,
                    path
                )}" ${getVisibility()}></i>
                <i class="fa fa-redo fa-lg paramReset" name="${name}${postFix}" path="${path}" default="${defaultVal}" defaultText="${defaultText}" )" ></i>
            </div>`;
        };

        function getHelp(name, path, defaultVal) {
            return getI18n(path + "_" + name).description;
        }

        function getAdvancedClass(advanced) {
            let advancedClass = "";
            if (advanced) {
                advancedClass = "advanced";
            }
            return advancedClass;
        }

        function resetToDefault(name, path, defaultVal) {
            console.log("reset", path, name, $("#" + path + "_" + name).prop("type"));

            if (
                $("#" + path + "_" + name).prop("type") == "checkbox" ||
                $("#" + path + "_" + name).prop("type") == "radio"
            ) {
                if (defaultVal == "true") {
                    if ($("#" + path + "_" + name).prop("type") == "radio") {
                        $("#" + path + "_" + name)
                            .parent()
                            .parent()
                            .children()
                            .filter(".active")
                            .removeClass("active");
                    }
                    $("#" + path + "_" + name).prop("checked", true);
                } else {
                    $("#" + path + "_" + name).prop("checked", false);
                }
            } else {
                $("#" + path + "_" + name)
                    .val(defaultVal)
                    .trigger("input");
            }
            $("#" + path + "_" + name).change();
        }

        function getMinMaxLabel(node) {
            if (node !== undefined && (node.content.min == null || node.content.max == null)) {
                return "";
            } else {
                return `(${node.content.min}-${node.content.max})`;
            }
        }

        function showRestartConfirmDialog() {
            return new Promise((resolve, reject) => {
                const compiledTemplate = _.template(restartConfirm);

                const template = compiledTemplate(revertRestartI18n);
                $("#confirmModal").remove();
                $("body").append(template);
                $(document).ready(() => {
                    $("#confirmModal").modal({
                        backdrop: "static",
                        keyboard: false,
                    });
                    $("#confirmModal").on("hidden.bs.modal", (e) => {
                        resolve(false);
                    });
                    $("#okRestartButton").click(() => {
                        resolve(true);

                        //disable future confirmation
                        if ($("#confirmRestartCheckbox").prop("checked")) {
                            const confirm = $("#_root_extra_restartConfirmDialog").prop(
                                "checked",
                                false
                            );
                            confirm.change();
                        }

                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                    $("#cancelRestartButton").click(() => {
                        resolve(false);
                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                });
            });
        }

        function showResetConfirmDialog(defaultVal, defaultText) {
            return new Promise((resolve, reject) => {
                const compiledTemplate = _.template(revertConfirm);

                revertRestartI18n.settingDefault = defaultText;

                const template = compiledTemplate(revertRestartI18n);
                $("#confirmModal").remove();
                $("body").append(template);
                $(document).ready(() => {
                    $("#confirmModal").modal({
                        backdrop: "static",
                        keyboard: false,
                    });
                    $("#confirmModal").on("hidden.bs.modal", (e) => {
                        resolve(false);
                    });
                    $("#okRevertButton").click(() => {
                        resolve(true);
                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                    $("#cancelRevertButton").click(() => {
                        resolve(false);
                        $("#confirmModal").modal("hide");
                        $("#confirmModal").remove();
                    });
                });
            });
        }

        function clampNumeric(element, value) {
            if (element.attr("min") !== "null" && element.attr("max") !== "null") {
                return _.clamp(value, element.attr("min"), element.attr("max"));
            } else {
                return value;
            }
        }

        function getNumericGuiType(nodeContent) {
            let guiType = nodeContent.gui;
            if (guiType == null) {
                if (nodeContent.min != null && nodeContent.max != null) {
                    if (nodeContent.step != null) {
                        guiType = "slider";
                    } else {
                        guiType = "updown";
                    }
                } else {
                    guiType = "textbox";
                }
            }
            return guiType;
        }

        init();
    };
});
