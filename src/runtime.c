// 2026 COPYRIGHT BY "VULPIN SOFTWARE.CC" ALL RIGHT REVERSED.
// MIT LICENCE

#include <stdio.h>
#include <string.h>

void run_vulpin(const char *filename, char *output_buffer, int buffer_size) {
    char command[512];
    snprintf(command, sizeof(command), "python vul.py %s 2>&1", filename);
    FILE *fp = popen(command, "r");
    if (fp != NULL) {
        // Read the output and put it into the buffer send to Python
        fgets(output_buffer, buffer_size, fp);
        pclose(fp);
    } else {
        strncpy(output_buffer, "Error running script", buffer_size);
    }
}
