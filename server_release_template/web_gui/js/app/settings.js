define([
    "json!../../settings-schema",
    "json!../../session",
    "app/customSettings",
    "lib/lodash",
    "i18n!app/nls/settings",
    "i18n!app/nls/revertRestart",
    "text!app/templates/revertConfirm.html",
    "text!app/templates/restartConfirm.html",

], function (schema, session, CustomSettings, _, i18n, revertRestartI18n, revertConfirm, restartConfirm) {
    return function () {
        var self = this;
       
        var advanced = false;
        var updating = false;
        var customSettings = new CustomSettings(self);
        var index = 0;
        const usedi18n = {};

        function randomAlphanumericID() {
            const len = 10;
            const arr = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghilmnopqrstuvwxyz0123456789";
            var ans = '';
            for (var i = len; i > 0; i--) {
                ans += arr[Math.floor(Math.random() * arr.length)];
            }
            return ans;
        }

        var webClientId = randomAlphanumericID();

        self.disableWizard = function () {
            session.setupWizard = false;
            self.storeSession("other");
        }

        self.updateClientTrustState = function (sessionListIndex, state) {
            session.lastClients[sessionListIndex].state = state;
            self.storeSession("clientList");
        }

        self.pushManualClient = function (descriptor) {
            session.lastClients.push(descriptor);
            self.storeSession("clientList");
        }

        self.removeClient = function (sessionListIndex) {
            session.lastClients.splice(sessionListIndex, 1);
            self.storeSession("clientList");
        }

        function init() {

            fillNode(schema, "root", 0, $("#configContent"), "", undefined);
            updateSwitchContent();
            updateOptionalContent();

            setProperties(session.settingsCache, "_root");

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
            setProperties(newSession.settingsCache, "_root");
            updating = false;
        }

        self.isUpdating = function () {
            return updating;
        }

        self.getSession = function () {
            return session;
        }

        function printUnusedi18n() {
            for (var key in i18n) {
                if (usedi18n[key] === undefined)
                    console.log("Unused i18n key:", key)
            }
        }

        function getI18n(id) {
            if (i18n === undefined) {
                console.log("names not ready");
                return { "name": id, "description": "" };
            } else {
                let name;
                if (i18n[id + ".name"] !== undefined) {
                    usedi18n[id + ".name"] = true;
                    name = i18n[id + ".name"]
                } else {
                    name = id.substring(id.lastIndexOf("_") + 1, id.length).replace("-choice-", "");
                }

                return { "name": name, "description": i18n[id + ".description"] };
            }
        }

        function addChangeListener() {
            $('.parameter input:not(.skipInput)').change((evt) => {
                if (!updating) {
                    var el = $(evt.target);
                    self.storeParam(el);
                }

            })
        }

        self.storeParam = function (el, skipstoreSession = false) {
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

            if (!skipstoreSession) {
                self.storeSession("settings");
            }
        }

        self.storeSession = function (updateType) {
            if (updating) {
                return;
            }

            $.ajax({
                type: "POST",
                url: `../../session/${updateType}/${webClientId}`,
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
                if (checked) {
                    $(el).find(".card-body").show();
                } else {
                    $(el).find(".card-body").hide();
                }
            })
        }

        function updateOptionalContent() {

            $(".optional").each((index, el) => {
                var checked = $(el).find("input[type='checkbox']").first().prop("checked");

                if (checked) {
                    $(el).find(".optionalSet").button("toggle");
                    $(el).find(".card-body").show();
                } else {
                    $(el).find(".optionalUnset").button("toggle");
                    $(el).find(".card-body").hide();
                }
            })
        }



        function addListeners() {
            $("#toggleAdvanced").click(() => {
                advanced = !advanced;
                toggleAdvanced();
            })

            $("#restartSteamVR").click(() => {
                restartSteamVR();
            })

            $(".paramReset").click((evt) => {
                var el = $(evt.target);

                var name = el.attr("name");
                var path = el.attr("path");
                var def = el.attr("default");

                if (!$("#" + path + "_" + name).prop("disabled")) {
                    const confirm = $("#_root_extra_revertConfirmDialog").prop("checked");
                    if (confirm) {
                        showResetConfirmDialog(def).then((res) => {
                            if (res) {
                                resetToDefault(name, path, def);
                            }
                        });
                    } else {
                        resetToDefault(name, path, def);
                    }
                }
            })
        }

        function addHelpTooltips() {
            $('[data-toggle="tooltip"]').tooltip()
        }

        function restartSteamVR() {
            const triggerRestart = () => {
                $.get("restart_steamvr", undefined, (res) => {
                    if (res == 0) {
                        Lobibox.notify("success", {
                            size: "mini",
                            rounded: true,
                            delayIndicator: false,
                            sound: false,
                            msg: i18n.steamVRRestartSuccess
                        })
                    }
                })
            }

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
            })

            //special cases like device dropdown
            $("#configContainer .special").each((index, el) => {
                if (advanced) {
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
                    case "headsetEmulationMode":
                    case "controllerMode":
                        addDropdown(element, path, name, advanced)
                        break;
                    case "suppressFrameDrop":
                    case "disableThrottling":
                        addBooleanType(element, path, name, advanced, { content: { default: false } });
                        break;
                    case "bufferOffset":
                        addNumericType(element, path, name, advanced, { content: { default: 0, gui: "slider" } })
                        break;

                    default:
                        console.log("Unhandled node without content. Should be implemented as special case:", name);
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
                        if (parentType != "switch" && parentType != "optional") { //switch and optional add own sections
                            element = addContainer(element, path, name, advanced);
                        }
                    }

                    var newPath = path + "_" + name;
                    if (parentType == "array") {
                        newPath = path;
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

                    if (level == 1) {
                        element.append(`<div class="button-spacer"></div>`)
                    }

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

                case "optional":
                    if (level == 1) {
                        element = createTab(element, path, name, advanced);
                        element = addOptionalContainer(element, path, name, advanced, node);
                    } else if (level > 1) {
                        element = addOptionalContainer(element, path, name, advanced, node);
                    }

                    fillNode(node.content.content, name, level + 1, element, path, node.type, node.content.advanced);
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

            //check if the tab is the first, then set classes to activate. First child is advanced button, second the reload steamVr
            if ($("#configContent").children().length == 3) {
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

        function addOptionalContainer(element, path, name, advanced, node) {

            let checked = "";
            if (node.content.defaultSet) {
                checked = "checked";
            }

            var el = `<div class="parameter optional ${getAdvancedClass(advanced)}" >   
                <div class="card-title">
                    <div class="btn-group btn-group-sm" data-toggle="buttons">
                        <label class="btn btn-primary optionalSet"><input class="skipInput" type="radio" name="${path}_${name}" id="${path}_${name}_setRadio" >Set</label>
                        <label class="btn btn-primary optionalUnset"><input class="skipInput" type="radio" name="${path}_${name}" id="${path}_${name}_unsetRadio" >Unset</label>               
                    </div>
                    <input  id="${path}_${name}_set" type="checkbox" ${checked}  style="visibility:hidden" />
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">
                    ${getI18n(path + "_" + name).name}</a> 
                    ${self.getHelpReset(name + "_set", path, node.content.defaultSet)}
                </div>   
                <div id="collapse_${index}" class="collapse show">
                    <div class="card-body">
                    </div>      
                </div> 
            </div>`;

            element.append(el);

            $(document).ready(() => {


                $("#" + path + "_" + name + "_setRadio").parent().click(() => {
                    $("#" + path + "_" + name + "_set").prop("checked", true)
                    $("#" + path + "_" + name + "_set").change();
                })
                $("#" + path + "_" + name + "_unsetRadio").parent().click(() => {
                    $("#" + path + "_" + name + "_set").prop("checked", false);
                    $("#" + path + "_" + name + "_set").change();

                })

            });

            $("#" + path + "_" + name + "_set").on("change", updateOptionalContent);

            element = element.find(".card-body").last();

            return element;
        }

        function addRadioContainer(element, path, name, advanced, node) {           
            var el = `<div class="parameter ${getAdvancedClass(advanced)}" >
                <div class="card-title">
                    ${getI18n(path + "_" + name + "-choice-").name}  ${self.getHelpReset(name + "_" + node.content.default + "-choice-", path, true)}
                </div>   
                <div>
                <form id="${path + '_' + name + '-choice-'}">
                    <div class="card-body">
                    </div>
                </form>
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
                        <label for="${path}_${name}">${getI18n(path + "_" + name).name} ${getMinMaxLabel(node)} </label>
                         ${self.getHelpReset(name, path, node.content.default)}                         
                    </div>`);
        }

        function addNumericType(element, path, name, advanced, node) {
            let type = getNumericGuiType(node.content);

            let base = `<div class="parameter ${getAdvancedClass(advanced)}" >
                    <label for="${path}_${name}">${getI18n(path + "_" + name).name} ${getMinMaxLabel(node)}: 
                    </label>`;

            switch (type) {
                case "slider":
                    base += `<div class="rangeValue" id="${path}_${name}_label">[${node.content.default}]</div>${self.getHelpReset(name, path, node.content.default)}
            <input numericType="${node.type}" id="${path}_${name}" type="range" min="${node.content.min}" 
            max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}"  >`;
                    break;

                case "upDown":
                case "updown":
                    var el = `<input numericType="${node.type}" id="${path}_${name}" type="number" min="${node.content.min}" 
                    max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}">`;
                    
                    var grp = `<div class="upDownGrp" ><div class="input-group">
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
                    base += ` <input numericType="${node.type}" id="${path}_${name}"  type="text" min="${node.content.min}" guiType="numeric" 
            max="${node.content.max}" value="${node.content.default}"  step="${node.content.step}" > ${self.getHelpReset(name, path, node.content.default)}`;
                    break;

                default:
                    console.log("numeric type was: ", type)

            }

            element.append(base + `</div>`);

            $("#" + path + "_" + name).on("input", (el) => {
                $("#" + el.target.id + "_label").text("[" + el.target.value + "]")
            });



            //add spinner functions
            $("#" + path + "_" + name + "[type=number]" ).prev().on("click", (el) => {
                var val = new Number($("#" + path + "_" + name).val());
                var step = 1;
                if(node.content.step !== null) {
                    step = node.content.step;
                }

                val = val - step;

                if(node.content.min != null && val < node.content.min) {
                    val = node.content.min;
                }
                $("#" + path + "_" + name).val(val);          
                $("#" + path + "_" + name).change();   

            });

            $("#" + path + "_" + name + "[type=number]" ).next().on("click", (el) => {
                var val = new Number($("#" + path + "_" + name).val());

                var step = 1;
                if(node.content.step !== null) {
                    step = node.content.step;
                }

                val = val + step;    

                if(node.content.max != null && val > node.content.max) {
                    val = node.content.max;
                }
                $("#" + path + "_" + name).val(val);
                $("#" + path + "_" + name).change();
            });
        }

        //helper
        self.getHelpReset = function (name, path, defaultVal, postFix = "") {
            var getVisibility = function() {
                if(getHelp(name, path) === undefined) {
                    return `style="display:none"`;
                }
            }
            return `<div class="helpReset">
                <i class="fa fa-question-circle fa-lg helpIcon" data-toggle="tooltip" title="${getHelp(name, path)}" ${getVisibility()}></i>
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
            if (node !== undefined && (node.content.min == null || node.content.max == null)) {
                return "";
            } else {
                return `(${node.content.min}-${node.content.max})`
            }
        }

        function showRestartConfirmDialog() {
            return new Promise((resolve, reject) => {
                var compiledTemplate = _.template(restartConfirm);

                var template = compiledTemplate(revertRestartI18n);
                $("#confirmModal").remove();
                $("body").append(template);
                $(document).ready(() => {

                    $('#confirmModal').modal({
                        backdrop: 'static',
                        keyboard: false
                    });
                    $('#confirmModal').on('hidden.bs.modal', (e) => {
                        resolve(false)
                    })
                    $("#okRestartButton").click(() => {
                        resolve(true)

                        //disable future confirmation
                        if ($("#confirmRestartCheckbox").prop("checked")) {
                            const confirm = $("#_root_extra_restartConfirmDialog").prop("checked", false);
                            confirm.change();
                        }

                        $('#confirmModal').modal('hide');
                        $('#confirmModal').remove();
                    })
                    $("#cancelRestartButton").click(() => {
                        resolve(false)
                        $('#confirmModal').modal('hide');
                        $('#confirmModal').remove();
                    })
                });
            });
        }


        function showResetConfirmDialog(defaultVal) {
            return new Promise((resolve, reject) => {
                var compiledTemplate = _.template(revertConfirm);
                revertRestartI18n.settingDefault = defaultVal;

                var template = compiledTemplate(revertRestartI18n);
                $("#confirmModal").remove();
                $("body").append(template);
                $(document).ready(() => {

                    $('#confirmModal').modal({
                        backdrop: 'static',
                        keyboard: false
                    });
                    $('#confirmModal').on('hidden.bs.modal', (e) => {
                        resolve(false)
                    })
                    $("#okRevertButton").click(() => {
                        resolve(true)
                        $('#confirmModal').modal('hide');
                        $('#confirmModal').remove();
                    })
                    $("#cancelRevertButton").click(() => {
                        resolve(false)
                        $('#confirmModal').modal('hide');
                        $('#confirmModal').remove();
                    })
                });
            });
        }

        function clampNumeric(element, value) {
            if (element.attr("min") !== "null" && element.attr("max") !== "null") {
                return _.clamp(value, element.attr("min"), element.attr("max"))
            } else {
                return value;
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


