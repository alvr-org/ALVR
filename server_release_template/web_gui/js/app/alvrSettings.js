
define([
    "json!../../settings-schema",
    "json!../../session",
    "lib/lodash",
    "i18n!app/nls/locale"


], function (schema, session, _, i18n) {
    return function () {
        var advanced = false;
        var updating = false;

        this.disableWizard = function () {
            session.setupWizard = false;
            updateSession();
        }

        function init() {
            targetSettings = session;
            fillNode(schema, "Main", 0, $("#configContent"), "root", undefined);
            updateSwitchContent();
            toggleAdvanced();
            addListeners();
            addHelpTooltips();
            setProperties(session.settingsCache, "root_Main");
            addChangeListener();

        }

        function getI18n(id) {

            if (i18n === undefined) {
                console.log("names not ready");
                return { "name": id, "description": "" };
            } else {
                if (i18n[id + ".name"] !== undefined) {
                    return { "name": i18n[id + ".name"], "description": i18n[id + ".description"] };;
                } else {
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
            id = id.replace("root_Main_", "");
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
                type: "PUT",
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
                        setProperties(res.settingsCache, "root_Main");
                        updating = false;
                    }
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
                        pathItem = object[item]
                    }

                    const el = $("#" + path + "_" + pathItem);

                    if (el.length == 0) {
                        console.log("NOT FOUND")
                        console.log("setting value: ", path + "_" + item, object[item])
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


        var index = 0;
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
        }


        function fillNode(node, name, level, element, path, parentType, advanced = false) {

            index += 1;

            //console.log(level, path + "_" + name, parentType)

            if (node == null) {
                return;
            }

            switch (node.type) {

                case "section":

                    //section in level 1
                    if (level == 1) {
                        element = createTab(path, name, advanced);

                    } else if (level > 1) {

                        if (parentType != "switch") { //switch adds section
                            element = addContainer(index, element, name, advanced);
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
                        element = createTab(path, name, advanced);
                        element = addSwitchContainer(index, element, name, node, path, advanced);
                    } else if (level > 1) {
                        element = addSwitchContainer(index, element, name, node, path, advanced);
                    }

                    fillNode(node.content.content, name, level + 1, element, path, node.type, node.content.advanced);
                    break;

                case "array":
                    element = addContainer(index, element, name, advanced);
                    node.content.forEach((el, index) => {
                        var arrayName = name + "_" + index


                        fillNode(el, arrayName, level + 1, element, path + "_" + arrayName, "array", el.advanced);

                    });

                    break;

                case "choice":

                    element = addRadioContainer(index, element, name, advanced, path, node);
                    node.content.variants.forEach((el, index) => {

                        var variantElement = addRadioVariant(element, el[0], name, el[1], path + "_" + name, el[0] == node.content.default);

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
                    addNumericType(element, name, node, path, advanced);
                    break;

                case "boolean":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addBooleanType(element, name, node, path, advanced);
                    break;

                case "text":
                    if (parentType == "choice" || parentType == "array") {
                        path = path.replace("_" + name, "");
                    }
                    addTextType(element, name, node, path, advanced);
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

        function createTab(path, name, advanced) {

            $("#configTabs").append(`
                    <li class="nav-item ${getAdvancedClass(advanced)}">
                        <a class="nav-link" data-toggle="tab" href="#${path + "_" + name}" id="${path + "_" + name + "_tab"}">${getI18n(name).name}</a>
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

        function addContainer(index, element, name, advanced) {

            var el = `<div class="parameter ${getAdvancedClass(advanced)}">
                <div class="card-title">
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">${getI18n(name).name}</a>
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

        function addRadioContainer(index, element, name, advanced, path, node) {
            var el = `<div class="parameter ${getAdvancedClass(advanced)}" >
                <div class="card-title">
                    ${getI18n(name).name}  ${getHelpReset(name + "_" + node.content.default, path, true)}
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


        function addRadioVariant(element, name, radioName, node, path, isDefault, advanced) {
            let checked = "";
            if (isDefault) {
                checked = "checked";
            }

            var el = `<div class="${getAdvancedClass(advanced)}" >
                <input type="radio" id="${path}_${name}" name="${radioName}"  value="${name}" ${checked}> 
                <label for="${path}_${name}">${getI18n(name).name}</label>
                <div class="radioContent">
                </div>
            </div>`;

            element.append(el);
            element = element.find(".radioContent").last();
            return element;
        }

        function addSwitchContainer(index, element, name, node, path, advanced) {
            let checked = "";
            if (node.content.defaultEnabled) {
                checked = "checked";
            }

            var el = `<div class="parameter switch ${getAdvancedClass(advanced)}" >   
                <div class="card-title">
                    <input id="${path}_${name}_enabled" type="checkbox" ${checked} " />
                    <a class="accordion-toggle" data-toggle="collapse" data-target="#collapse_${index}" href="#collapse_${index}" aria-expanded="true">
                    ${getI18n(name).name}</a> 
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

        function addTextType(element, name, node, path, advanced) {
            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" >     
                        <label for="${path}_${name}">${getI18n(name).name} </label> 
                        ${getHelpReset(name, path, node.content.default)}
                        <input id="${path}_${name}" type="text" value="${node.content.default}" >
                        </input>
                    </div>`);
        }

        function addBooleanType(element, name, node, path, advanced) {
            let checked = "";
            if (node.content.default) {
                checked = "checked";
            }

            element.append(`<div class="parameter ${getAdvancedClass(advanced)}" > 
                        <input id="${path}_${name}" type="checkbox" ${checked} />
                        <label for="${path}_${name}">${getI18n(name).name} ${getMinMaxLabel(node)} </label>
                         ${getHelpReset(name, path, node.content.default)}                         
                    </div>`);
        }


        function addNumericType(element, name, node, path, advanced) {
            let type = getNumericGuiType(node.content);

            let base = `<div class="parameter ${getAdvancedClass(advanced)}" >
                    <label for="${path}_${name}">${getI18n(name).name} ${getMinMaxLabel(node)}: 
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



        function getHelpReset(name, path, defaultVal) {
            return `<div class="helpReset">
                <i class="fa fa-question-circle fa-lg helpIcon" data-toggle="tooltip" title="${getHelp(name, path)}" ></i>
                <i class="fa fa-redo fa-lg paramReset" name="${name}" path="${path}" default="${defaultVal}")" ></i>
            </div>`;
        }

        function getHelp(name, path, defaultVal) {
            return getI18n(name).description;
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


