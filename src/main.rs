#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(cruster::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::string::String;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use futures_util::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use cruster::{print, println};
use cruster::task::keyboard::ScancodeStream;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::{structures::paging::Page, VirtAddr};

    use cruster::allocator;
    use cruster::memory;
    use cruster::memory::BootInfoFrameAllocator;
    use cruster::task::executor::Executor;
    use cruster::task::Task;

    println!("Hello World{}", "!");

    cruster::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // map an unused page
    let page = Page::containing_address(VirtAddr::new(0xdeadbeaf000));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);


    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");


    let mut executor = Executor::new();
    executor.spawn(Task::new(shell()));
    executor.run();

    #[cfg(test)]
    test_main();

    cruster::hlt_loop();
}

async fn input(stream: &mut ScancodeStream) -> String {
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    let mut inp = String::new();

    while inp.chars().last() != Some('\n') {
        if let Some(scancode) = stream.next().await {
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => {
                            print!("{}", character);
                            inp.push(character);
                        },
                        DecodedKey::RawKey(key) => println!("{:?}", key),
                    }
                }
            }
        }
    }

    inp.chars().take_while(|ch| ch != &'\n').collect()
}

async fn shell() {
    let mut stream = ScancodeStream::new();

    loop {
        print!("> ");
        let inp = input(&mut stream).await;

        if !inp.is_empty() {
            match inp.as_str() {
                "help" => println!("This is a simple help string"),
                _ => println!("Unrecognised command"),
            }
        }
    }
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    cruster::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cruster::test_panic_handler(info)
}
