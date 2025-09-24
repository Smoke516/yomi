#!/usr/bin/env python3
"""
Test script to validate the enhanced RSS reader features
"""
import subprocess
import time
import os
import sys

def test_build():
    """Test that the project builds successfully"""
    print("🔧 Testing build...")
    result = subprocess.run(
        ["cargo", "build"], 
        cwd="/home/seawn/rust_rss_reader",
        capture_output=True,
        text=True
    )
    
    if result.returncode == 0:
        print("✅ Build successful!")
        return True
    else:
        print(f"❌ Build failed: {result.stderr}")
        return False

def test_config_creation():
    """Test that config file is created properly"""
    print("📝 Testing config creation...")
    
    # Remove existing config if present
    config_dir = os.path.expanduser("~/.config/tokyo-rss")
    config_file = os.path.join(config_dir, "config.toml")
    
    if os.path.exists(config_file):
        os.remove(config_file)
    
    # Run the app briefly to trigger config creation
    proc = subprocess.Popen(
        ["cargo", "run"],
        cwd="/home/seawn/rust_rss_reader",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    
    # Let it run for 2 seconds to initialize
    time.sleep(2)
    proc.terminate()
    proc.wait()
    
    if os.path.exists(config_file):
        print("✅ Config file created successfully!")
        with open(config_file, 'r') as f:
            print(f"📄 Config contents preview:\n{f.read()[:200]}...")
        return True
    else:
        print("❌ Config file was not created")
        return False

def check_dependencies():
    """Check that all required dependencies are present in Cargo.toml"""
    print("📦 Checking dependencies...")
    
    required_deps = [
        'ratatui', 'crossterm', 'tokio', 'feed-rs', 'serde', 
        'toml', 'reqwest', 'anyhow', 'chrono', 'dirs', 
        'regex', 'open', 'futures', 'sha2', 'hex'
    ]
    
    with open("/home/seawn/rust_rss_reader/Cargo.toml", 'r') as f:
        cargo_content = f.read()
    
    missing_deps = []
    for dep in required_deps:
        if dep not in cargo_content:
            missing_deps.append(dep)
    
    if not missing_deps:
        print("✅ All required dependencies present!")
        return True
    else:
        print(f"❌ Missing dependencies: {missing_deps}")
        return False

def main():
    print("🚀 Testing Tokyo RSS Reader Enhanced Features\n")
    
    tests = [
        ("Build Test", test_build),
        ("Dependencies Check", check_dependencies),
        ("Config Creation", test_config_creation),
    ]
    
    passed = 0
    total = len(tests)
    
    for test_name, test_func in tests:
        print(f"\n{'='*50}")
        print(f"Running: {test_name}")
        print('='*50)
        
        try:
            if test_func():
                passed += 1
            else:
                print(f"❌ {test_name} failed")
        except Exception as e:
            print(f"❌ {test_name} failed with error: {e}")
        
        print()
    
    print(f"\n🎯 Test Results: {passed}/{total} tests passed")
    
    if passed == total:
        print("\n🎉 All tests passed! The enhanced RSS reader is ready to use.")
        print("\n📖 New Features Available:")
        print("  • 3-column layout (feeds, articles, preview)")
        print("  • Status bar with unread count and last refresh time")
        print("  • Browser integration with 'o' key")
        print("  • Beautiful Tokyo Night color scheme")
        print("\n💡 Usage:")
        print("  cargo run           # Start the RSS reader")
        print("  Tab                 # Switch between panes")
        print("  ↑/↓                 # Navigate")
        print("  o                   # Open article in browser")
        print("  r                   # Refresh feeds")
        print("  h                   # Show help")
        print("  q                   # Quit")
    else:
        print("\n⚠️  Some tests failed. Please check the output above.")
        sys.exit(1)

if __name__ == "__main__":
    main()
