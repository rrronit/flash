#include <iostream>
using namespace std;

int main() {
    // Declare variables
    int num1, num2;
    int sum, difference, product;
    float quotient;

    // Input: Ask the user for two numbers
    num1 = 0;
    num2 = 11;

    // Perform arithmetic operations
    sum = num1 + num2;
    difference = num1 - num2;
    product = num1 * num2;

    // Check if the second number is not zero before division
    if (num2 != 0) {
        quotient = static_cast<float>(num1) / num2;
    } else {
        cout << "Division by zero is not allowed!" << endl;
        return 1; // Exit the program with an error code
    }

    // Output: Display the results
    cout << "Sum: " << sum << endl;
    cout << "Difference: " << difference << endl;
    cout << "Product: " << product << endl;
    cout << "Quotient: " << quotient << endl;

    return 0; // Exit the program successfully
}