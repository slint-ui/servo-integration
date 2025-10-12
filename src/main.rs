fn main() {
    #[cfg(not(target_os = "android"))]
    servo_integration_lib::main();

    #[cfg(target_os = "android")]
    servo_integration_lib::android_main();
}
