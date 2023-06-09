/*
    SpifyRFB - Modern RFB Server implementation using Rust
    Copyright (C) 2023  Atheesh Thirumalairajan

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use windows::Win32::UI::Input::KeyboardAndMouse as Win32_KeyboardAndMouse;

pub fn create_keysym_vk_map() -> HashMap<u32, Win32_KeyboardAndMouse::VIRTUAL_KEY> {
    HashMap::from([
        /* TTY Function Keys */
        (0xff08, Win32_KeyboardAndMouse::VK_BACK),
        (0xff09, Win32_KeyboardAndMouse::VK_TAB),
        (0xff0d, Win32_KeyboardAndMouse::VK_RETURN),
        (0xff13, Win32_KeyboardAndMouse::VK_PAUSE),
        (0xff1b, Win32_KeyboardAndMouse::VK_ESCAPE),
        (0xffff, Win32_KeyboardAndMouse::VK_DELETE),

        /* MOTION KEYS */
        (0xff50, Win32_KeyboardAndMouse::VK_HOME),
        (0xff51, Win32_KeyboardAndMouse::VK_LEFT),
        (0xff52, Win32_KeyboardAndMouse::VK_UP),
        (0xff53, Win32_KeyboardAndMouse::VK_RIGHT),
        (0xff54, Win32_KeyboardAndMouse::VK_DOWN),
        (0xff55, Win32_KeyboardAndMouse::VK_PRIOR),
        (0xff56, Win32_KeyboardAndMouse::VK_NEXT),

        /* MISC FUNCTIONS */
        (0xff60, Win32_KeyboardAndMouse::VK_SELECT),
        (0xff61, Win32_KeyboardAndMouse::VK_PRINT),
        (0xff61, Win32_KeyboardAndMouse::VK_EXECUTE),
        (0xff63, Win32_KeyboardAndMouse::VK_INSERT),
        (0xff67, Win32_KeyboardAndMouse::VK_MENU),
        (0xff69, Win32_KeyboardAndMouse::VK_CANCEL),
        (0xff6a, Win32_KeyboardAndMouse::VK_HELP),
        (0xff7f, Win32_KeyboardAndMouse::VK_NUMLOCK),

        /* KEYPAD FUNCTIONS */
        (0xff80, Win32_KeyboardAndMouse::VK_SPACE),
        (0xff89, Win32_KeyboardAndMouse::VK_TAB),
        (0xff8d, Win32_KeyboardAndMouse::VK_RETURN),
        (0xff91, Win32_KeyboardAndMouse::VK_F1),
        (0xff91, Win32_KeyboardAndMouse::VK_F2),
        (0xff93, Win32_KeyboardAndMouse::VK_F3),
        (0xff94, Win32_KeyboardAndMouse::VK_F4),
        (0xff95, Win32_KeyboardAndMouse::VK_HOME),
        (0xff95, Win32_KeyboardAndMouse::VK_LEFT),
        (0xff97, Win32_KeyboardAndMouse::VK_UP),
        (0xff98, Win32_KeyboardAndMouse::VK_RIGHT),
        (0xff99, Win32_KeyboardAndMouse::VK_DOWN),
        (0xff9a, Win32_KeyboardAndMouse::VK_PRIOR),
        (0xff9b, Win32_KeyboardAndMouse::VK_NEXT),
        (0xff9c, Win32_KeyboardAndMouse::VK_END),
        (0xff9e, Win32_KeyboardAndMouse::VK_INSERT),
        (0xff9e, Win32_KeyboardAndMouse::VK_DELETE),
        (0xffbd, Win32_KeyboardAndMouse::VK_OEM_NEC_EQUAL),
        (0xffaa, Win32_KeyboardAndMouse::VK_MULTIPLY),
        (0xffaa, Win32_KeyboardAndMouse::VK_ADD),
        (0xffac, Win32_KeyboardAndMouse::VK_SEPARATOR),
        (0xffad, Win32_KeyboardAndMouse::VK_SUBTRACT),
        (0xffae, Win32_KeyboardAndMouse::VK_DECIMAL),
        (0xffaf, Win32_KeyboardAndMouse::VK_DIVIDE),
        (0xffb0, Win32_KeyboardAndMouse::VK_NUMPAD0),
        (0xffb1, Win32_KeyboardAndMouse::VK_NUMPAD1),
        (0xffb2, Win32_KeyboardAndMouse::VK_NUMPAD2),
        (0xffb3, Win32_KeyboardAndMouse::VK_NUMPAD3),
        (0xffb4, Win32_KeyboardAndMouse::VK_NUMPAD4),
        (0xffb5, Win32_KeyboardAndMouse::VK_NUMPAD5),
        (0xffb6, Win32_KeyboardAndMouse::VK_NUMPAD6),
        (0xffb7, Win32_KeyboardAndMouse::VK_NUMPAD7),
        (0xffb8, Win32_KeyboardAndMouse::VK_NUMPAD8),
        (0xffb9, Win32_KeyboardAndMouse::VK_NUMPAD9),

        /* AUXILARY FUNCTION KEYS */
        (0xffbe, Win32_KeyboardAndMouse::VK_F1),
        (0xffbf, Win32_KeyboardAndMouse::VK_F2),
        (0xffc0, Win32_KeyboardAndMouse::VK_F3),
        (0xffc1, Win32_KeyboardAndMouse::VK_F4),
        (0xffc2, Win32_KeyboardAndMouse::VK_F5),
        (0xffc3, Win32_KeyboardAndMouse::VK_F6),
        (0xffc4, Win32_KeyboardAndMouse::VK_F7),
        (0xffc5, Win32_KeyboardAndMouse::VK_F8),
        (0xffc6, Win32_KeyboardAndMouse::VK_F9),
        (0xffc7, Win32_KeyboardAndMouse::VK_F10),
        (0xffc8, Win32_KeyboardAndMouse::VK_F11),
        (0xffc9, Win32_KeyboardAndMouse::VK_F12),
        (0xffca, Win32_KeyboardAndMouse::VK_F13),
        (0xffcb, Win32_KeyboardAndMouse::VK_F14),
        (0xffcc, Win32_KeyboardAndMouse::VK_F15),
        (0xffcd, Win32_KeyboardAndMouse::VK_F16),
        (0xffce, Win32_KeyboardAndMouse::VK_F17),
        (0xffcf, Win32_KeyboardAndMouse::VK_F18),
        (0xffd0, Win32_KeyboardAndMouse::VK_F19),
        (0xffd1, Win32_KeyboardAndMouse::VK_F20),
        (0xffd2, Win32_KeyboardAndMouse::VK_F21),
        (0xffd3, Win32_KeyboardAndMouse::VK_F22),
        (0xffd4, Win32_KeyboardAndMouse::VK_F23),
        (0xffd5, Win32_KeyboardAndMouse::VK_F24),

        /* MODIFIERS */
        (0xffe1, Win32_KeyboardAndMouse::VK_LSHIFT),
        (0xffe2, Win32_KeyboardAndMouse::VK_RSHIFT),
        (0xffe3, Win32_KeyboardAndMouse::VK_LCONTROL),
        (0xffe4, Win32_KeyboardAndMouse::VK_RCONTROL),
        (0xffe5, Win32_KeyboardAndMouse::VK_CAPITAL),
        (0xffe7, Win32_KeyboardAndMouse::VK_LWIN),
        (0xffe8, Win32_KeyboardAndMouse::VK_RWIN),
        (0xffe9, Win32_KeyboardAndMouse::VK_LMENU),
        (0xffea, Win32_KeyboardAndMouse::VK_RMENU),
        (0xffeb, Win32_KeyboardAndMouse::VK_LWIN),
        (0xffec, Win32_KeyboardAndMouse::VK_RWIN),
        (0xffed, Win32_KeyboardAndMouse::VK_LWIN),
        (0xffee, Win32_KeyboardAndMouse::VK_RWIN),

        /* ISO AND TERMINAL KEYS */
        (0xfd1d, Win32_KeyboardAndMouse::VK_SNAPSHOT),
        (0xfd16, Win32_KeyboardAndMouse::VK_PLAY),

        /* LATIN-1 KEYS */
        (0x0020, Win32_KeyboardAndMouse::VK_SPACE),
        (0x0030, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x30)), /* 0 to 9 */
        (0x0031, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x31)),
        (0x0032, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x32)),
        (0x0033, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x33)),
        (0x0034, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x34)),
        (0x0035, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x35)),
        (0x0036, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x36)),
        (0x0037, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x37)),
        (0x0038, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x38)),
        (0x0039, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x39)),
        
        /* A to Z */
        (0x0041, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x41)),
        (0x0042, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x42)),
        (0x0043, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x43)),
        (0x0044, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x44)),
        (0x0045, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x45)),
        (0x0046, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x46)),
        (0x0047, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x47)),
        (0x0048, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x48)),
        (0x0049, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x49)),
        (0x004a, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4a)),
        (0x004b, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4b)),
        (0x004c, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4c)),
        (0x004d, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4d)),
        (0x004e, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4e)),
        (0x004f, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4f)),
        (0x0050, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x50)),
        (0x0051, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x51)),
        (0x0052, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x52)),
        (0x0053, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x53)),
        (0x0054, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x54)),
        (0x0055, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x55)),
        (0x0056, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x56)),
        (0x0057, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x57)),
        (0x0058, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x58)),
        (0x0059, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x59)),
        (0x005a, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x5a)),

        /* a to z */
        (0x0061, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x41)),
        (0x0062, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x42)),
        (0x0063, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x43)),
        (0x0064, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x44)),
        (0x0065, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x45)),
        (0x0066, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x46)),
        (0x0067, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x47)),
        (0x0068, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x48)),
        (0x0069, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x49)),
        (0x006a, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4a)),
        (0x006b, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4b)),
        (0x006c, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4c)),
        (0x006d, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4d)),
        (0x006e, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4e)),
        (0x006f, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x4f)),
        (0x0070, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x50)),
        (0x0071, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x51)),
        (0x0072, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x52)),
        (0x0073, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x53)),
        (0x0074, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x54)),
        (0x0075, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x55)),
        (0x0076, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x56)),
        (0x0077, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x57)),
        (0x0078, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x58)),
        (0x0079, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x59)),
        (0x007a, Win32_KeyboardAndMouse::VIRTUAL_KEY(0x5a)),
    ])
}