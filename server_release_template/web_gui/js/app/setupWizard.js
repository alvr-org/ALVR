define([
    "lib/lodash",
    "text!app/templates/wizard.html",
    "css!app/templates/wizard.css"
], function (_, wizardTemplate) {
    return function () {
        console.log("Wizard")

        var compiledTemplate = _.template(wizardTemplate);
        var test = compiledTemplate({ name: 'moe' });

        $("body").append(test);
        $(document).ready(() => {
            $('#setupWizard').modal({
                backdrop: 'static',
                keyboard: false
            });

        });


    };
});