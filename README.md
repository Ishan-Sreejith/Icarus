# Icarus (formerly FrameForge)

A Modular, High-Performance Language for Digital Architects.

Icarus is a modern, colon-driven programming language built for clarity, speed, and deep customization. By separating logic and data into specialized "Frames" and "Forges," it allows developers to build scalable systems without the overhead of traditional OOP.

# Core Philosophy
Structural Clarity: Clear separation between local, global, and high-performance (Fast) logic.

Action-Oriented Syntax: Uses a consistent keyword: data flow that reads like a story.

Modular by Design: Everything is a component that can be forged, refined, and reused.

# Quick Start
Installation
Bash
./forge install
Your First Program
Create a file named main.fr:


Language Tour
# 1. Variables & Data Types
FrameForge uses : for state assignment and for function parameters.


var metadata = {"key": "value"} # Map
var list = [1, 2, 3]            # List
# 2. The Hierarchy of Logic (Functions)
FrameForge categorizes functions based on their scope and performance needs.
| Keyword | Type | Best For... |
| :--- | :--- | :--- |
| fn | Local | Standard logic within a module. |
| fng | Global | Tools and utilities accessible everywhere. |
| fnc | Fast | High-performance, math-heavy, or "hot" code paths. |

Code snippet
fnc add_fast: a, b {
    return a + b
}
# 3. The Hierarchy of Data (Classes)
Classes follow the same tiered logic as functions, allowing for memory optimization.
| Keyword | Type | Description |
| :--- | :--- | :--- |
| cl | Standard | Typical object-oriented data structures. |
| clg | Global | Persistent data that lives for the app's lifetime. |
| clc | Fast/Compact | Optimized for low memory footprint and speed. |


# Development Status
FrameForge is currently in active development.
