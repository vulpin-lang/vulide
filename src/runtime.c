#include <stdio.h>
#include <tcl.h>

// This function runs the Vulpin file saved in the IDE folder
// NOTE: @ line 12, if Vulpin becomes a PyPI package, change the command.

void run_vulpin_file(Tcl_Interp *interp, const char *filename, const char *widgetName) {
    char command[512];
    char buffer[1024];

    // Build the command string (python vul.py file.vul 2>&1)
    snprintf(command, sizeof(command), "python vul.py %s 2>&1", filename);

    // Open the pipe
    FILE *fp = popen(command, "r");

    if (fp == NULL) {
        // Report error to the user via the console widget
        Tcl_VarEval(interp, widgetName, " insert end \"Error: Could not start runner\n\"", NULL);
        return;
    }

    // Read output and pipe it to the widget...
    while (fgets(buffer, sizeof(buffer), fp) != NULL) {
        // Dynamically build the Tcl command: widgetName insert end "output_text"
        // Use Tcl_VarEval to assemble it
        Tcl_VarEval(interp, widgetName, " insert end \"", buffer, "\"", NULL);
        
        // Auto-scroll the widget
        Tcl_VarEval(interp, widgetName, " see end", NULL);
    }

    pclose(fp);
}
