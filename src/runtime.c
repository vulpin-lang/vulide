#include <stdio.h>

int main() {
    char text[100];

    scanf("%99s", text);        // Read from Python
    printf("Hello %s\n", text); // Send back to Python

    return 0;
}
