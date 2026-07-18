// Copyright (c) 2026 Vulpin Software. All rights reserved.
// SPDX-License-Identifier: MIT

void run_vulpin(const char *filename, char *output_buffer, int buffer_size) {
    char command[512];
    snprintf(command, sizeof(command), "python vul.py %s 2>&1", filename);

    FILE *fp = popen(command, "r");
    if (fp != NULL) {
        // Read the output and put it into the buffer we'll send to Python
        fgets(output_buffer, buffer_size, fp);
        pclose(fp);
    } else {
        strncpy(output_buffer, "Error running script", buffer_size);
    }
}
