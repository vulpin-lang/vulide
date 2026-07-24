# 2026 COPYRIGHT BY VULPIN LABS ALL RIGHT REVERSED.

import sys
import os
import json
import shutil
import tempfile

from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QTextEdit, QPlainTextEdit, QWidget,
    QTabWidget, QVBoxLayout, QHBoxLayout, QToolBar, QAction, QFileDialog,
    QMessageBox, QStatusBar, QLabel, QComboBox, QDockWidget, QSplitter,
    QMenu, QCompleter, QDialog, QLineEdit, QCheckBox, QPushButton,
    QTreeWidget, QTreeWidgetItem, QFontDialog, QStyleFactory,
    QGraphicsView, QGraphicsScene, QGraphicsObject, QGraphicsPathItem,
    QGraphicsEllipseItem, QGraphicsItem, QInputDialog, QListWidget,
    QListWidgetItem, QGraphicsTextItem, QGraphicsRectItem, QGraphicsLineItem,
    QTabBar, QSpinBox, QGroupBox, QFormLayout, QSlider, QRadioButton,
    QButtonGroup, QDialogButtonBox, QScrollArea
)

from PyQt5.QtGui import (
    QFont, QColor, QTextCharFormat, QSyntaxHighlighter, QPalette,
    QTextCursor, QPainter, QKeySequence, QTextDocument,
    QBrush, QFontMetrics, QDesktopServices, QTextFormat, QPen,
    QPainterPath, QLinearGradient, QConicalGradient
)

from PyQt5.QtCore import (
    Qt, QRect, QSize, QRegularExpression, QTimer, QUrl, pyqtSignal,
    QRectF, QPoint, QPointF, QProcess
)

THEME_KEYS = [
    "bg", "fg", "gutter_bg", "gutter_border", "line_fg", "line_hl",
    "sel", "current_line", "match_bracket", "comment", "string",
    "number", "keyword", "command", "control", "variable", "operator",
    "function", "bracket", "builtin", "toolbar_bg", "toolbar_fg",
    "tab_bg", "tab_active", "output_bg", "output_fg", "output_err",
    "output_ok", "dock_bg", "dock_fg", "menu_bg", "menu_fg",
    "statusbar_bg", "statusbar_fg", "scrollbar", "scrollbar_hover",
    "autocomplete_bg", "autocomplete_fg", "autocomplete_sel", "accent"
]

THEME_DATA = {
    "Dark (Catppuccin Mocha)": [
        "#1e1e2e", "#cdd6f4", "#181825", "#313244", "#6c7086", "#f5e0dc",
        "#45475a", "#2a2a3c", "#89b4fa", "#6c7086", "#a6e3a1", "#fab387",
        "#89b4fa", "#89b4fa", "#cba6f7", "#f9e2af", "#89dceb", "#f5c2e7",
        "#94e2d5", "#f38ba8", "#181825", "#cdd6f4", "#1e1e2e", "#313244",
        "#11111b", "#a6adc8", "#f38ba8", "#a6e3a1", "#1e1e2e", "#cdd6f4",
        "#1e1e2e", "#cdd6f4", "#181825", "#a6adc8", "#45475a", "#585b70",
        "#313244", "#cdd6f4", "#45475a", "#89b4fa"
    ],
    "Light (Catppuccin Latte)": [
        "#eff1f5", "#4c4f69", "#e6e9ef", "#ccd0da", "#9ca0b0", "#dc8a78",
        "#bcc0cc", "#dce0e8", "#1e66f5", "#9ca0b0", "#40a02b", "#fe640b",
        "#1e66f5", "#1e66f5", "#8839ef", "#df8e1d", "#179299", "#ea76cb",
        "#179299", "#d20f39", "#e6e9ef", "#4c4f69", "#eff1f5", "#ccd0da",
        "#dce0e8", "#5c5f77", "#d20f39", "#40a02b", "#eff1f5", "#4c4f69",
        "#eff1f5", "#4c4f69", "#e6e9ef", "#5c5f77", "#bcc0cc", "#9ca0b0",
        "#ccd0da", "#4c4f69", "#bcc0cc", "#1e66f5"
    ],
    "Dracula": [
        "#282a36", "#f8f8f2", "#21222c", "#44475a", "#6272a4", "#f8f8f0",
        "#44475a", "#343746", "#8be9fd", "#6272a4", "#f1fa8c", "#bd93f9",
        "#8be9fd", "#8be9fd", "#ff79c6", "#ffb86c", "#ff79c6", "#50fa7b",
        "#f8f8f2", "#bd93f9", "#21222c", "#f8f8f2", "#282a36", "#44475a",
        "#21222c", "#6272a4", "#ff5555", "#50fa7b", "#282a36", "#f8f8f2",
        "#282a36", "#f8f8f2", "#21222c", "#6272a4", "#44475a", "#6272a4",
        "#44475a", "#f8f8f2", "#6272a4", "#bd93f9"
    ],
    "Nord": [
        "#2e3440", "#d8dee9", "#272c36", "#3b4252", "#4c566a", "#eceff4",
        "#434c5e", "#353b49", "#88c0d0", "#4c566a", "#a3be8c", "#b48ead",
        "#81a1c1", "#81a1c1", "#bf616a", "#ebcb8b", "#81a1c1", "#88c0d0",
        "#eceff4", "#d08770", "#272c36", "#d8dee9", "#2e3440", "#3b4252",
        "#272c36", "#4c566a", "#bf616a", "#a3be8c", "#2e3440", "#d8dee9",
        "#2e3440", "#d8dee9", "#272c36", "#4c566a", "#434c5e", "#4c566a",
        "#3b4252", "#d8dee9", "#434c5e", "#88c0d0"
    ],
    "Solarized Dark": [
        "#002b36", "#839496", "#00222b", "#073642", "#586e75", "#93a1a1",
        "#073642", "#073642", "#268bd2", "#586e75", "#859900", "#d33682",
        "#268bd2", "#268bd2", "#cb4b16", "#b58900", "#2aa198", "#268bd2",
        "#93a1a1", "#d33682", "#00222b", "#839496", "#002b36", "#073642",
        "#00222b", "#586e75", "#dc322f", "#859900", "#002b36", "#839496",
        "#002b36", "#839496", "#00222b", "#586e75", "#073642", "#586e75",
        "#073642", "#839496", "#073642", "#268bd2"
    ],
    "Monokai": [
        "#272822", "#f8f8f2", "#1e1f1c", "#3e3d32", "#75715e", "#f8f8f0",
        "#49483e", "#33342e", "#66d9ef", "#75715e", "#e6db74", "#ae81ff",
        "#66d9ef", "#66d9ef", "#f92672", "#fd971f", "#f92672", "#a6e22e",
        "#f8f8f2", "#ae81ff", "#1e1f1c", "#f8f8f2", "#272822", "#3e3d32",
        "#1e1f1c", "#a59f85", "#f92672", "#a6e22e", "#272822", "#f8f8f2",
        "#272822", "#f8f8f2", "#1e1f1c", "#a59f85", "#49483e", "#75715e",
        "#3e3d32", "#f8f8f2", "#49483e", "#66d9ef"
    ],
}

THEMES = {name: dict(zip(THEME_KEYS, values)) for name, values in THEME_DATA.items()}

VULPIN_COMMANDS = {
    "G": ("Print", 'G expr', "Print expression with newline"),
    "P": ("Print (no nl)", 'P expr', "Print expression without newline"),
    "A": ("Arithmetic assign", 'A"var"op expr', "var = var op expr"),
    "S": ("String replace", 'S"var""old""new"', "Replace substring in variable"),
    "D": ("Delay / Delete", 'D seconds / D"var"', "Wait or delete a variable"),
    "K": ("Input", 'K"var""prompt""type"', "Read input from keyboard"),
    "X": ("Execute file", 'X"file.py"', "Run Python file in background"),
    "Q": ("Quit", 'Q', "Exit the program"),
    "E": ("Error exit", 'E"msg"', "Print error message and exit"),
    "U": ("Import", 'U"module"', "Import Python module or .vul file"),
    "O": ("For-range", 'O var start end [step]', "Counted loop"),
    "L": ("Label", 'L name', "Define a jump label"),
    "J": ("Jump", 'J label', "Unconditional jump"),
    "F": ("Function", 'F name(params)', "Define a function"),
    "R": ("Return", 'R expr', "Return from function"),
    "T": ("Try", 'T', "Start try block"),
    "C": ("Catch", 'C / C"var"', "Catch exception"),
    "Y": ("End try", 'Y', "End try/catch block"),
    "W": ("Switch", 'W expr', "Start switch block"),
    "V": ("Case", 'V value', "Case in switch"),
    "N": ("Default", 'N', "Default case"),
    "Z": ("End switch", 'Z', "End switch block"),
    "!": ("Python exec", '! code', "Execute raw Python code"),
}

VULPIN_CONTROL = {
    "?": ("If / Cond jump", '? cond', "Conditional execution"),
    ":": ("Else", ':', "Else clause"),
    ";": ("Endif", ';', "End if block"),
    "@": ("While", '@ cond', "Start while loop"),
    "&": ("Wend / End for", '&', "End loop"),
    "~": ("End function", '~', "End function body"),
}

VULPIN_SNIPPETS = {
    "if": '? $cond\n    \n:',
    "while": '@ $cond\n    \n&',
    "for": 'O i 0 10\n    \n&',
    "func": 'F name(params)\n    \n~',
    "try": 'T\n    \nC"err"\n    G $err\nY',
    "switch": 'W $expr\n    V val1\n        \n    N\n        \nZ',
    "print": 'G""',
    "input": 'K"var""prompt: ""S"',
    "import": 'U"module"',
    "label": 'L name',
    "jump": 'J name',
}

BLOCK_OPENERS = {"?", "@", "O", "F", "T", "W", "V", "N", ":"}

BLUEPRINTS = {
    "Basics": [
        ("Hello World", 'G"Hello, World!"'),
        ("Print Message", 'G"Your message here"'),
        ("Print Variable", 'name="Vulpin"\nG"Name: " + $name'),
        ("Set Variable", 'x=10'),
        ("Quit", 'G"Bye!"\nQ'),
    ],
    "Input": [
        ("Text Input", 'K"answer""Enter text: ""S"\nG"You entered: " + $answer'),
        ("Number Input", 'K"num""Enter number: ""N"\nG"Number: " + $num'),
    ],
    "Logic": [
        ("If", 'x=15\n? $x > 10\n    G"Big"\n;'),
        ("If Else", 'x=15\n? $x > 10\n    G"Big"\n:\n    G"Small"\n;'),
        ("Switch", 'x=1\nW $x\n    V 1\n        G"One"\n    N\n        G"Other"\nZ'),
    ],
    "Loops": [
        ("While", 'i=0\n@ $i < 5\n    G $i\n    i=$i+1\n&'),
        ("For", 'O i 0 5\n    G $i\n&'),
    ],
    "Functions": [
        ("Function", 'F add(a,b)\n    R $a + $b\n~\nG $add(2,3)'),
        ("Recursion", 'F factorial(n)\n    ? $n <= 1\n        R 1\n    ;\n    R $n * $factorial($n - 1)\n~\nG $factorial(5)'),
    ],
    "Error Handling": [
        ("Try Catch", 'T\n    x=10/0\nC"err"\n    G"Error: " + $err\nY'),
    ],
    "Data": [
        ("String Replace", 'msg="hello"\nS"msg""hello""world"\nG $msg'),
        ("Arithmetic", 'x=5\nA"x"+3\nG $x'),
    ],
    "System": [
        ("Delay", 'D 1'),
        ("Import", 'U"random"'),
        ("Python Exec", '! import datetime\nG str(datetime.datetime.now())'),
    ],
}

CATEGORY_COLORS = {
    "Basics": "#4a88c7",
    "Input": "#d08770",
    "Logic": "#cc7832",
    "Loops": "#a3be8c",
    "Functions": "#b48ead",
    "Error Handling": "#bf616a",
    "Data": "#88c0d0",
    "System": "#ebcb8b",
}

DEFAULT_SETTINGS = {
    "theme": "Dark (Catppuccin Mocha)",
    "recent_files": [],
    "font_size": 12,
    "font_family": "JetBrains Mono",
    "tab_width": 4,
    "word_wrap": False,
    "auto_close_brackets": True,
    "auto_indent": True,
    "show_autocomplete": True,
    "show_line_numbers": True,
    "auto_save": False,
    "auto_save_interval": 60,
    "recent_files_limit": 10,
    "confirm_on_close": True,
    "restore_session": False,
    "power_profile": "balanced",
    "algo_refresh_ms": 400,
    "autocomplete_delay_ms": 250,
    "vulpin_path": "",
    "terminal_shell": "",
    "output_height": 220,
}

POWER_PROFILES = {
    "power_save": {
        "algo_refresh_ms": 1000,
        "autocomplete_delay_ms": 500,
        "description": "Lower CPU usage, slower refresh rates",
    },
    "balanced": {
        "algo_refresh_ms": 400,
        "autocomplete_delay_ms": 250,
        "description": "Balanced performance and power usage",
    },
    "performance": {
        "algo_refresh_ms": 100,
        "autocomplete_delay_ms": 100,
        "description": "Fastest response, higher CPU usage",
    },
}


class PinItem(QGraphicsEllipseItem):
    def __init__(self, node, pin_type, radius=7):
        super().__init__(-radius, -radius, radius * 2, radius * 2, node)
        self.node = node
        self.pin_type = pin_type
        self.radius = radius
        self.connections = []
        self.theme = node.theme
        self.setAcceptHoverEvents(True)
        self.setAcceptedMouseButtons(Qt.NoButton)
        self.setZValue(4)
        self.set_theme(self.theme)

    def set_theme(self, theme):
        self.theme = theme
        if self.pin_type == "out":
            self.setBrush(QColor(theme.get("accent", "#89b4fa")))
            self.setPen(QPen(QColor(theme["fg"]), 1.4))
        else:
            self.setBrush(QColor(theme["bg"]))
            self.setPen(QPen(QColor(theme.get("accent", "#89b4fa")), 1.8))
        self.update()

    def update_connections(self):
        for conn in self.connections:
            conn.update_path()

    def remove_all_connections(self):
        for conn in list(self.connections):
            conn.remove_from_pins()
            if self.scene():
                self.scene().removeItem(conn)
        self.connections.clear()

    def hoverEnterEvent(self, event):
        self.setCursor(Qt.CrossCursor)
        super().hoverEnterEvent(event)


class ConnectionItem(QGraphicsPathItem):
    def __init__(self, source_pin, target_pin=None, color=None, temp=False):
        super().__init__()
        self.source_pin = source_pin
        self.target_pin = target_pin
        self.temp = temp
        self.temp_end = QPointF()
        self.color = color or QColor("#89b4fa")
        self.setZValue(5 if temp else 1)
        self.setFlag(QGraphicsItem.ItemIsSelectable, True)
        self.setBrush(QBrush(Qt.NoBrush))
        style = Qt.DashLine if temp else Qt.SolidLine
        self.setPen(QPen(self.color, 2.2, style, Qt.RoundCap, Qt.RoundJoin))
        if source_pin and not temp:
            source_pin.connections.append(self)
        if target_pin:
            target_pin.connections.append(self)
        self.update_path()

    def set_color(self, color):
        self.color = color
        pen = self.pen()
        pen.setColor(color)
        self.setPen(pen)
        self.update()

    def set_temp_end(self, pos):
        self.temp_end = pos

    def update_path(self):
        if not self.source_pin:
            return
        start = self.source_pin.scenePos()
        end = self.target_pin.scenePos() if self.target_pin else self.temp_end
        if end is None:
            return
        sign = 1 if end.x() >= start.x() else -1
        dx = max(abs(end.x() - start.x()) * 0.5, 70)
        path = QPainterPath()
        path.moveTo(start)
        path.cubicTo(start.x() + sign * dx, start.y(), end.x() - sign * dx, end.y(), end.x(), end.y())
        self.setPath(path)

    def remove_from_pins(self):
        if self.source_pin and self in self.source_pin.connections:
            self.source_pin.connections.remove(self)
        if self.target_pin and self in self.target_pin.connections:
            self.target_pin.connections.remove(self)


class BlueprintNodeItem(QGraphicsObject):
    def __init__(self, title, category, code, theme, width=235):
        super().__init__()
        self.title = title
        self.category = category
        self.code = code
        self.theme = theme
        self.width = width
        self.title_height = 30
        self.pin_y = 54
        self.height = 102
        self.setFlag(QGraphicsItem.ItemIsMovable, True)
        self.setFlag(QGraphicsItem.ItemIsSelectable, True)
        self.setFlag(QGraphicsItem.ItemSendsGeometryChanges, True)
        self.setZValue(3)
        self.category_color = QColor(CATEGORY_COLORS.get(category, theme.get("accent", "#89b4fa")))
        self.pin_in = PinItem(self, "in")
        self.pin_out = PinItem(self, "out")
        self.pins = [self.pin_in, self.pin_out]
        self._layout_pins()

    def _layout_pins(self):
        in_pins = [p for p in self.pins if p.pin_type == "in"]
        out_pins = [p for p in self.pins if p.pin_type == "out"]
        for i, pin in enumerate(in_pins):
            pin.setPos(0, self.pin_y + i * 22)
        for i, pin in enumerate(out_pins):
            pin.setPos(self.width, self.pin_y + i * 22)
        needed = self.pin_y + max(len(in_pins), len(out_pins)) * 22 + 34
        self.height = max(102, int(needed))
        self.update()

    def add_pin(self, pin_type):
        pin = PinItem(self, pin_type)
        self.pins.append(pin)
        self._layout_pins()
        return pin

    def remove_all_connections(self):
        for pin in self.pins:
            pin.remove_all_connections()

    def boundingRect(self):
        return QRectF(-14, -14, self.width + 28, self.height + 28)

    def paint(self, painter, option, widget=None):
        painter.setRenderHint(QPainter.Antialiasing, True)
        body = QPainterPath()
        body.addRoundedRect(0, 0, self.width, self.height, 9, 9)
        body_color = QColor(self.theme["toolbar_bg"])
        body_color.setAlpha(235)
        painter.fillPath(body, body_color)
        title_path = QPainterPath()
        title_path.addRoundedRect(0, 0, self.width, self.title_height, 9, 9)
        title_path.addRect(0, self.title_height - 9, self.width, 9)
        painter.fillPath(title_path, self.category_color)
        if self.isSelected():
            border_color = QColor(self.theme.get("accent", "#89b4fa"))
            border_width = 2.0
        else:
            border_color = QColor(self.theme["gutter_border"])
            border_width = 1.2
        painter.setPen(QPen(border_color, border_width))
        painter.setBrush(QBrush(Qt.NoBrush))
        painter.drawPath(body)
        title_font = QFont("Segoe UI", 10, QFont.Bold)
        small_font = QFont("Segoe UI", 9)
        code_font = QFont("JetBrains Mono", 9)
        painter.setFont(title_font)
        painter.setPen(self._contrast_text(self.category_color))
        painter.drawText(QRectF(12, 0, self.width - 24, self.title_height), Qt.AlignLeft | Qt.AlignVCenter, self.title)
        painter.setFont(code_font)
        painter.setPen(QColor(self.theme["fg"]))
        preview = self.code.strip().split("\n")[0]
        fm = QFontMetrics(code_font)
        elided = fm.elidedText(preview, Qt.ElideRight, int(self.width - 30))
        painter.drawText(QRectF(14, self.title_height + 6, self.width - 28, 20), Qt.AlignLeft | Qt.AlignVCenter, elided)
        painter.setFont(small_font)
        painter.setPen(QColor(self.theme["line_fg"]))
        painter.drawText(QRectF(14, self.pin_y - 9, 70, 18), Qt.AlignLeft | Qt.AlignVCenter, "In")
        painter.drawText(QRectF(self.width - 70, self.pin_y - 9, 56, 18), Qt.AlignRight | Qt.AlignVCenter, "Out")
        painter.drawText(QRectF(14, self.height - 22, self.width - 28, 16), Qt.AlignRight | Qt.AlignVCenter, self.category)

    def itemChange(self, change, value):
        if change == QGraphicsItem.ItemPositionHasChanged:
            self.update_connections()
        return super().itemChange(change, value)

    def update_connections(self):
        for pin in self.pins:
            pin.update_connections()

    def set_theme(self, theme):
        self.theme = theme
        for pin in self.pins:
            pin.set_theme(theme)
        self.update()

    def mouseDoubleClickEvent(self, event):
        scene = self.scene()
        if scene and hasattr(scene, "node_edit_requested"):
            scene.node_edit_requested.emit(self)
        event.accept()

    def _contrast_text(self, color):
        yiq = (color.red() * 299 + color.green() * 587 + color.blue() * 114) / 1000
        return QColor("#101010") if yiq >= 145 else QColor("#f5f5f5")


class BlueprintScene(QGraphicsScene):
    node_edit_requested = pyqtSignal(object)

    def __init__(self, theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self.temp_connection = None
        self.setSceneRect(-2500, -2500, 5000, 5000)

    def set_theme(self, theme):
        self.theme = theme
        self.setBackgroundBrush(QColor(theme["bg"]))
        accent = QColor(theme.get("accent", "#89b4fa"))
        for item in self.items():
            if isinstance(item, BlueprintNodeItem):
                item.set_theme(theme)
            elif isinstance(item, ConnectionItem):
                item.set_color(accent)
            elif isinstance(item, PinItem):
                item.set_theme(theme)
        self.update()

    def add_blueprint_node(self, title, category, code, pos):
        node = BlueprintNodeItem(title, category, code, self.theme)
        self.addItem(node)
        node.setPos(pos)
        return node

    def add_connection(self, source_pin, target_pin):
        if not source_pin or not target_pin or source_pin.node == target_pin.node:
            return None
        color = QColor(self.theme.get("accent", "#89b4fa"))
        conn = ConnectionItem(source_pin, target_pin, color, False)
        self.addItem(conn)
        return conn

    def start_temp_connection(self, pin):
        if self.temp_connection:
            self.finish_temp_connection(None)
        color = QColor(self.theme.get("accent", "#89b4fa"))
        self.temp_connection = ConnectionItem(pin, None, color, True)
        self.temp_connection.set_temp_end(pin.scenePos())
        self.temp_connection.update_path()
        self.addItem(self.temp_connection)

    def update_temp_connection(self, pos):
        if self.temp_connection:
            self.temp_connection.set_temp_end(pos)
            self.temp_connection.update_path()

    def finish_temp_connection(self, target_pin):
        if not self.temp_connection:
            return
        source_pin = self.temp_connection.source_pin
        self.removeItem(self.temp_connection)
        self.temp_connection = None
        if target_pin and isinstance(target_pin, PinItem) and target_pin.pin_type == "in" and target_pin.node != source_pin.node:
            self.add_connection(source_pin, target_pin)

    def delete_selected(self):
        items = self.selectedItems()
        for item in items:
            if isinstance(item, ConnectionItem):
                item.remove_from_pins()
                if item.scene():
                    self.removeItem(item)
        for item in items:
            if isinstance(item, BlueprintNodeItem):
                item.remove_all_connections()
                self.removeItem(item)

    def generate_code(self):
        nodes = [item for item in self.items() if isinstance(item, BlueprintNodeItem)]
        if not nodes:
            return ""
        node_ids = {id(node): node for node in nodes}
        adj = {id(node): [] for node in nodes}
        indeg = {id(node): 0 for node in nodes}
        seen = set()
        for item in self.items():
            if isinstance(item, ConnectionItem) and item.source_pin and item.target_pin:
                s = item.source_pin.node
                t = item.target_pin.node
                key = (id(s), id(t))
                if key in seen or id(s) not in adj or id(t) not in indeg:
                    continue
                seen.add(key)
                adj[id(s)].append(id(t))
                indeg[id(t)] += 1
        queue = [nid for nid, deg in indeg.items() if deg == 0]
        ordered = []
        while queue:
            queue.sort(key=lambda nid: (node_ids[nid].pos().y(), node_ids[nid].pos().x()))
            nid = queue.pop(0)
            ordered.append(nid)
            for nxt in adj[nid]:
                indeg[nxt] -= 1
                if indeg[nxt] == 0:
                    queue.append(nxt)
        if len(ordered) < len(nodes):
            remaining = [id(node) for node in nodes if id(node) not in ordered]
            remaining.sort(key=lambda nid: (node_ids[nid].pos().y(), node_ids[nid].pos().x()))
            ordered.extend(remaining)
        blocks = []
        for nid in ordered:
            code = node_ids[nid].code.strip()
            if code:
                blocks.append(code)
        return "\n\n".join(blocks)

    def drawBackground(self, painter, rect):
        painter.fillRect(rect, QColor(self.theme["bg"]))
        grid = 26
        pen = QPen(QColor(self.theme["gutter_border"]), 1)
        pen.setCosmetic(True)
        painter.setPen(pen)
        draw_rect = rect.intersected(self.sceneRect())
        left = int(draw_rect.left())
        top = int(draw_rect.top())
        right = int(draw_rect.right())
        bottom = int(draw_rect.bottom())
        MAX_COORD = 100000
        left = max(left, -MAX_COORD)
        top = max(top, -MAX_COORD)
        right = min(right, MAX_COORD)
        bottom = min(bottom, MAX_COORD)
        if left >= right or top >= bottom:
            return
        start_x = left - (left % grid)
        start_y = top - (top % grid)
        for x in range(start_x, right, grid):
            painter.drawLine(x, top, x, bottom)
        for y in range(start_y, bottom, grid):
            painter.drawLine(left, y, right, y)


class BlueprintView(QGraphicsView):
    def __init__(self, scene):
        super().__init__(scene)
        self.setRenderHints(QPainter.Antialiasing | QPainter.TextAntialiasing | QPainter.SmoothPixmapTransform)
        self.setViewportUpdateMode(QGraphicsView.FullViewportUpdate)
        self.setTransformationAnchor(QGraphicsView.AnchorUnderMouse)
        self.setDragMode(QGraphicsView.RubberBandDrag)
        self.setMouseTracking(True)
        self._panning = False
        self._pan_start = QPoint()
        self.setStyleSheet("QGraphicsView { border: none; }")

    def wheelEvent(self, event):
        factor = 1.15 if event.angleDelta().y() > 0 else 0.87
        self.scale(factor, factor)
        event.accept()

    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton:
            pin = self._pin_at(event.pos())
            if pin and pin.pin_type == "out":
                self.scene().start_temp_connection(pin)
                event.accept()
                return
        elif event.button() == Qt.MiddleButton:
            self._panning = True
            self._pan_start = event.pos()
            self.viewport().setCursor(Qt.ClosedHandCursor)
            event.accept()
            return
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if self.scene().temp_connection:
            self.scene().update_temp_connection(self.mapToScene(event.pos()))
            event.accept()
            return
        if self._panning:
            delta = event.pos() - self._pan_start
            self._pan_start = event.pos()
            self.horizontalScrollBar().setValue(self.horizontalScrollBar().value() - delta.x())
            self.verticalScrollBar().setValue(self.verticalScrollBar().value() - delta.y())
            event.accept()
            return
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        if event.button() == Qt.LeftButton and self.scene().temp_connection:
            pin = self._pin_at(event.pos())
            self.scene().finish_temp_connection(pin)
            event.accept()
            return
        if event.button() == Qt.MiddleButton:
            self._panning = False
            self.viewport().unsetCursor()
            event.accept()
            return
        super().mouseReleaseEvent(event)

    def keyPressEvent(self, event):
        if event.key() in (Qt.Key_Delete, Qt.Key_Backspace):
            self.scene().delete_selected()
            event.accept()
            return
        super().keyPressEvent(event)

    def _pin_at(self, pos):
        point = self.mapToScene(pos)
        for item in self.scene().items(point):
            if isinstance(item, PinItem):
                return item
        return None


class BlueprintsDock(QDockWidget):
    insertRequested = pyqtSignal(str)
    syncCodeRequested = pyqtSignal(str)

    def __init__(self, theme, parent=None):
        super().__init__("Blueprints", parent)
        self.theme = theme
        self.setAllowedAreas(Qt.LeftDockWidgetArea | Qt.RightDockWidgetArea)
        self.setMinimumWidth(620)
        self._node_offset = 0
        self.auto_sync = False
        self._syncing = False
        self._sync_timer = QTimer()
        self._sync_timer.setSingleShot(True)
        self._sync_timer.setInterval(250)
        self._sync_timer.timeout.connect(self._emit_sync)
        self._build_ui()
        self._add_default_nodes()
        self.scene.changed.connect(lambda changes: self._schedule_sync())

    def _build_ui(self):
        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        toolbar = QWidget()
        h = QHBoxLayout(toolbar)
        h.setContentsMargins(6, 6, 6, 6)
        h.setSpacing(6)
        self.btn_add = QPushButton("Add Node")
        self.btn_add.setMenu(self._build_add_menu())
        self.btn_edit = QPushButton("Edit Node")
        self.btn_edit.clicked.connect(self._edit_selected_node)
        self.btn_add_in = QPushButton("Add In")
        self.btn_add_in.clicked.connect(lambda: self._add_pin_to_selected("in"))
        self.btn_add_out = QPushButton("Add Out")
        self.btn_add_out.clicked.connect(lambda: self._add_pin_to_selected("out"))
        self.btn_insert = QPushButton("Insert Graph")
        self.btn_insert.clicked.connect(self._insert_graph)
        self.btn_auto_sync = QPushButton("Auto Sync: Off")
        self.btn_auto_sync.clicked.connect(self._toggle_auto_sync)
        self.btn_clear = QPushButton("Reset")
        self.btn_clear.clicked.connect(self._clear)
        self.btn_reset = QPushButton("Reset View")
        self.btn_reset.clicked.connect(self._reset_view)
        h.addWidget(self.btn_add)
        h.addWidget(self.btn_edit)
        h.addWidget(self.btn_add_in)
        h.addWidget(self.btn_add_out)
        h.addWidget(self.btn_insert)
        h.addWidget(self.btn_auto_sync)
        h.addWidget(self.btn_clear)
        h.addWidget(self.btn_reset)
        h.addStretch()
        self.scene = BlueprintScene(self.theme)
        self.view = BlueprintView(self.scene)
        self.scene.node_edit_requested.connect(self._edit_node_object)
        layout.addWidget(toolbar)
        layout.addWidget(self.view)
        self.setWidget(container)

    def _build_add_menu(self):
        menu = QMenu(self)
        for category, items in BLUEPRINTS.items():
            submenu = menu.addMenu(category)
            for name, code in items:
                act = QAction(name, self)
                act.triggered.connect(lambda checked, n=name, c=category, cd=code: self.add_node(n, c, cd))
                submenu.addAction(act)
        return menu

    def add_node(self, title, category, code):
        center = self.view.mapToScene(self.view.viewport().rect().center())
        offset = self._node_offset * 35
        self._node_offset += 1
        self.scene.add_blueprint_node(title, category, code, QPointF(center.x() + offset, center.y() + offset))

    def _add_default_nodes(self):
        self.scene.clear()
        self.scene.temp_connection = None
        self._node_offset = 0
        n1 = self.scene.add_blueprint_node("Print Message", "Basics", BLUEPRINTS["Basics"][1][1], QPointF(-430, -130))
        n2 = self.scene.add_blueprint_node("Text Input", "Input", BLUEPRINTS["Input"][0][1], QPointF(-80, -80))
        n3 = self.scene.add_blueprint_node("If Else", "Logic", BLUEPRINTS["Logic"][1][1], QPointF(280, 20))
        self.scene.add_connection(n1.pin_out, n2.pin_in)
        self.scene.add_connection(n2.pin_out, n3.pin_in)

    def _selected_node(self):
        for item in self.scene.selectedItems():
            if isinstance(item, BlueprintNodeItem):
                return item
        return None

    def _edit_selected_node(self):
        node = self._selected_node()
        if not node:
            QMessageBox.information(self, "Edit Node", "Select a node first.")
            return
        text, ok = QInputDialog.getMultiLineText(self, "Edit Node", "Vulpin code:", node.code)
        if ok:
            node.code = text
            node.update()
            self._schedule_sync()

    def _edit_node_object(self, node):
        if not isinstance(node, BlueprintNodeItem):
            return
        text, ok = QInputDialog.getMultiLineText(self, "Edit Node", "Vulpin code:", node.code)
        if ok:
            node.code = text
            node.update()
            self._schedule_sync()

    def _add_pin_to_selected(self, pin_type):
        node = self._selected_node()
        if not node:
            QMessageBox.information(self, "Add Pin", "Select a node first.")
            return
        node.add_pin(pin_type)

    def _insert_graph(self):
        code = self.scene.generate_code()
        if code:
            self.insertRequested.emit(code + "\n")

    def _clear(self):
        self._add_default_nodes()

    def _reset_view(self):
        self.view.resetTransform()
        self.view.centerOn(0, 0)

    def _toggle_auto_sync(self):
        self.auto_sync = not self.auto_sync
        if self.auto_sync:
            self.btn_auto_sync.setText("Auto Sync: On")
            self._emit_sync()
        else:
            self.btn_auto_sync.setText("Auto Sync: Off")

    def _schedule_sync(self):
        if self.auto_sync and not self._syncing:
            self._sync_timer.start()

    def _emit_sync(self):
        if not self.auto_sync:
            return
        self._syncing = True
        code = self.scene.generate_code()
        self.syncCodeRequested.emit(code)
        self._syncing = False

    def set_theme(self, theme):
        self.theme = theme
        self.scene.set_theme(theme)


VB_CONTROLS = [
    ("Print", 'G"Hello"', "📝", "Label-like output"),
    ("Input", 'K"var""Prompt: ""S"', "📥", "Text input box"),
    ("Variable", 'x=10', "📦", "Variable declaration"),
    ("If", 'x=15\n? $x > 10\n    G"Big"\n;', "❓", "Conditional branch"),
    ("While", 'i=0\n@ $i < 5\n    G $i\n    i=$i+1\n&', "🔄", "While loop"),
    ("For", 'O i 0 10\n    G $i\n&', "🔁", "For loop"),
    ("Function", 'F name(params)\n    R $result\n~', "⚙️", "Function definition"),
    ("Try", 'T\n    \nC"err"\n    G $err\nY', "🛡️", "Try/Catch block"),
    ("Switch", 'x=1\nW $x\n    V 1\n        G"One"\n    N\n        G"Other"\nZ', "🔀", "Switch statement"),
    ("Delay", 'D 1', "⏱️", "Delay execution"),
    ("Import", 'U"module"', "📚", "Import module"),
    ("Quit", 'Q', "🚪", "Exit program"),
    ("Custom", '', "✏️", "Custom code"),
]


class VBControlItem(QGraphicsObject):
    """A Visual Basic-style control that adapts to the current theme."""
    
    def __init__(self, control_type, code, icon, description, theme, width=180, height=80):
        super().__init__()
        self.control_type = control_type
        self.code = code
        self.icon = icon
        self.description = description
        self.theme = theme
        self.width = width
        self.height = height
        self.title_height = 22
        self.setFlag(QGraphicsItem.ItemIsMovable, True)
        self.setFlag(QGraphicsItem.ItemIsSelectable, True)
        self.setFlag(QGraphicsItem.ItemSendsGeometryChanges, True)
        self.setZValue(2)
        self.setAcceptHoverEvents(True)
        self._hover = False

    def set_theme(self, theme):
        self.theme = theme
        self.update()

    def boundingRect(self):
        return QRectF(-4, -4, self.width + 8, self.height + 8)

    def _contrast_text(self, color):
        yiq = (color.red() * 299 + color.green() * 587 + color.blue() * 114) / 1000
        return QColor("#101010") if yiq >= 145 else QColor("#f5f5f5")

    def paint(self, painter, option, widget=None):
        painter.setRenderHint(QPainter.Antialiasing, False)
        t = self.theme
        
        # Theme-adaptive colors
        bg_color = QColor(t["toolbar_bg"])
        title_color = QColor(t["accent"])
        text_color = QColor(t["fg"])
        border_light = QColor(t["bg"]).lighter(140)
        border_dark = QColor(t["gutter_border"])
        border_darker = QColor(t["gutter_border"]).darker(130)
        
        if self.isSelected():
            bg_color = QColor(t["sel"])
            border_dark = QColor(t.get("accent", "#89b4fa"))
        
        # Outer dark border
        painter.setPen(QPen(border_darker, 1))
        painter.setBrush(Qt.NoBrush)
        painter.drawRect(0, 0, self.width - 1, self.height - 1)
        
        # Light highlight (top and left)
        painter.setPen(QPen(border_light, 1))
        painter.drawLine(1, 1, self.width - 2, 1)
        painter.drawLine(1, 1, 1, self.height - 2)
        
        # Dark shadow (bottom and right)
        painter.setPen(QPen(border_dark, 1))
        painter.drawLine(1, self.height - 2, self.width - 2, self.height - 2)
        painter.drawLine(self.width - 2, 1, self.width - 2, self.height - 2)
        
        # Fill background
        painter.fillRect(2, 2, self.width - 4, self.height - 4, bg_color)
        
        # Title bar
        painter.fillRect(2, 2, self.width - 4, self.title_height, title_color)
        
        # Title text
        painter.setPen(self._contrast_text(title_color))
        title_font = QFont("Segoe UI", 9, QFont.Bold)
        painter.setFont(title_font)
        title_text = f"{self.icon} {self.control_type}"
        painter.drawText(QRectF(6, 2, self.width - 12, self.title_height), 
                        Qt.AlignLeft | Qt.AlignVCenter, title_text)
        
        # Body content - code preview
        painter.setPen(text_color)
        body_font = QFont("JetBrains Mono", 9)
        painter.setFont(body_font)
        
        code_lines = self.code.split("\n")[:3]
        y = self.title_height + 6
        fm = QFontMetrics(body_font)
        for line in code_lines:
            elided = fm.elidedText(line, Qt.ElideRight, self.width - 16)
            painter.drawText(QRectF(6, y, self.width - 12, fm.height()),
                           Qt.AlignLeft | Qt.AlignVCenter, elided)
            y += fm.height() + 2
        
        # Selection handles
        if self.isSelected():
            self._draw_selection_handles(painter)

    def _draw_selection_handles(self, painter):
        handle_size = 6
        accent = QColor(self.theme.get("accent", "#89b4fa"))
        painter.setPen(QPen(QColor("#000000"), 1))
        painter.setBrush(QBrush(accent))
        
        positions = [
            (0, 0), (self.width // 2, 0), (self.width - handle_size, 0),
            (0, self.height // 2), (self.width - handle_size, self.height // 2),
            (0, self.height - handle_size), (self.width // 2, self.height - handle_size),
            (self.width - handle_size, self.height - handle_size),
        ]
        
        for x, y in positions:
            painter.drawRect(x, y, handle_size, handle_size)

    def itemChange(self, change, value):
        return super().itemChange(change, value)

    def hoverEnterEvent(self, event):
        self._hover = True
        self.update()
        super().hoverEnterEvent(event)

    def hoverLeaveEvent(self, event):
        self._hover = False
        self.update()
        super().hoverLeaveEvent(event)

    def mouseDoubleClickEvent(self, event):
        scene = self.scene()
        if scene and hasattr(scene, "control_edit_requested"):
            scene.control_edit_requested.emit(self)
        event.accept()


class VBFormScene(QGraphicsScene):
    control_edit_requested = pyqtSignal(object)
    control_selected = pyqtSignal(object)

    def __init__(self, theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self.setSceneRect(-1000, -1000, 2000, 2000)

    def set_theme(self, theme):
        self.theme = theme
        self.update()

    def drawBackground(self, painter, rect):
        t = self.theme
        
        # Workspace background
        painter.fillRect(rect, QColor(t["dock_bg"]))
        
        # Form area - slightly lighter than background
        form_rect = QRectF(0, 0, 640, 480)
        form_bg = QColor(t["bg"]).lighter(110) if self._is_dark_theme(t) else QColor(t["bg"]).darker(102)
        painter.fillRect(form_rect, form_bg)
        
        # Form border
        painter.setPen(QPen(QColor(t["gutter_border"]).darker(120), 2))
        painter.drawRect(form_rect)
        
        # Dotted grid on form
        grid_color = QColor(t["gutter_border"])
        painter.setPen(QPen(grid_color, 1, Qt.DotLine))
        grid_size = 8
        
        draw_rect = rect.intersected(form_rect)
        left = int(draw_rect.left())
        top = int(draw_rect.top())
        right = int(draw_rect.right())
        bottom = int(draw_rect.bottom())
        
        left = max(left, 0)
        top = max(top, 0)
        right = min(right, 640)
        bottom = min(bottom, 480)
        
        start_x = left - (left % grid_size)
        start_y = top - (top % grid_size)
        
        for x in range(start_x, right, grid_size):
            for y in range(start_y, bottom, grid_size):
                painter.drawPoint(x, y)
        
        # Form caption
        painter.setPen(QColor(t["fg"]))
        painter.setFont(QFont("Segoe UI", 10, QFont.Bold))
        painter.drawText(QRectF(10, 10, 300, 20), Qt.AlignLeft, "Form1 - Vulpin")

    def _is_dark_theme(self, theme):
        bg = QColor(theme["bg"])
        yiq = (bg.red() * 299 + bg.green() * 587 + bg.blue() * 114) / 1000
        return yiq < 128


class VBFormView(QGraphicsView):
    def __init__(self, scene):
        super().__init__(scene)
        self.setRenderHints(QPainter.Antialiasing | QPainter.TextAntialiasing)
        self.setViewportUpdateMode(QGraphicsView.FullViewportUpdate)
        self.setTransformationAnchor(QGraphicsView.AnchorUnderMouse)
        self.setDragMode(QGraphicsView.RubberBandDrag)
        self.setMouseTracking(True)
        self._panning = False
        self._pan_start = QPoint()
        self.setStyleSheet("QGraphicsView { border: none; }")

    def wheelEvent(self, event):
        factor = 1.15 if event.angleDelta().y() > 0 else 0.87
        self.scale(factor, factor)
        event.accept()

    def mousePressEvent(self, event):
        if event.button() == Qt.MiddleButton:
            self._panning = True
            self._pan_start = event.pos()
            self.viewport().setCursor(Qt.ClosedHandCursor)
            event.accept()
            return
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if self._panning:
            delta = event.pos() - self._pan_start
            self._pan_start = event.pos()
            self.horizontalScrollBar().setValue(self.horizontalScrollBar().value() - delta.x())
            self.verticalScrollBar().setValue(self.verticalScrollBar().value() - delta.y())
            event.accept()
            return
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        if event.button() == Qt.MiddleButton:
            self._panning = False
            self.viewport().unsetCursor()
            event.accept()
            return
        super().mouseReleaseEvent(event)

    def keyPressEvent(self, event):
        if event.key() in (Qt.Key_Delete, Qt.Key_Backspace):
            for item in self.scene().selectedItems():
                if isinstance(item, VBControlItem):
                    self.scene().removeItem(item)
            event.accept()
            return
        super().keyPressEvent(event)


class VBToolbox(QWidget):
    control_clicked = pyqtSignal(tuple)
    
    def __init__(self, theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self._build_ui()
        self._apply_theme()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(2, 2, 2, 2)
        layout.setSpacing(2)

        self.title = QLabel("Toolbox")
        layout.addWidget(self.title)

        self.list = QListWidget()
        self.list.setIconSize(QSize(20, 20))

        for ctrl_type, code, icon, desc in VB_CONTROLS:
            item = QListWidgetItem(f"{icon}  {ctrl_type}")
            item.setToolTip(desc)
            item.setData(Qt.UserRole, (ctrl_type, code, icon, desc))
            self.list.addItem(item)

        self.list.itemDoubleClicked.connect(self._on_item_clicked)
        layout.addWidget(self.list)

    def _apply_theme(self):
        t = self.theme
        self.title.setStyleSheet(f"""
            QLabel {{
                background: qlineargradient(x1:0, y1:0, x2:1, y2:0,
                    stop:0 {t['accent']}, stop:1 {QColor(t['accent']).lighter(130).name()});
                color: {'#101010' if self._is_light(t['accent']) else '#ffffff'};
                font-weight: bold;
                padding: 4px;
            }}
        """)
        self.list.setStyleSheet(f"""
            QListWidget {{
                background: {t['dock_bg']};
                color: {t['dock_fg']};
                border: 1px solid {t['gutter_border']};
                font-family: Segoe UI;
                font-size: 11px;
            }}
            QListWidget::item {{
                padding: 4px;
                border-bottom: 1px solid {t['gutter_border']};
            }}
            QListWidget::item:selected {{
                background: {t['sel']};
                color: {t['fg']};
            }}
            QListWidget::item:hover {{
                background: {t['autocomplete_sel']};
            }}
        """)

    def _is_light(self, color):
        c = QColor(color)
        yiq = (c.red() * 299 + c.green() * 587 + c.blue() * 114) / 1000
        return yiq >= 145

    def set_theme(self, theme):
        self.theme = theme
        self._apply_theme()

    def _on_item_clicked(self, item):
        data = item.data(Qt.UserRole)
        if data:
            self.control_clicked.emit(data)


class VBProperties(QWidget):
    code_changed = pyqtSignal(object, str)

    def __init__(self, theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self.current_control = None
        self._build_ui()
        self._apply_theme()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(2, 2, 2, 2)
        layout.setSpacing(2)

        self.title = QLabel("Properties")
        layout.addWidget(self.title)

        self.info_label = QLabel("No control selected")
        layout.addWidget(self.info_label)

        code_label = QLabel("Code:")
        self.code_label = code_label
        layout.addWidget(code_label)

        self.code_edit = QPlainTextEdit()
        self.code_edit.setFont(QFont("JetBrains Mono", 10))
        self.code_edit.textChanged.connect(self._on_code_changed)
        layout.addWidget(self.code_edit)

        self.apply_btn = QPushButton("Apply")
        self.apply_btn.clicked.connect(self._apply_code)
        layout.addWidget(self.apply_btn)

        layout.addStretch()

    def _apply_theme(self):
        t = self.theme
        self.title.setStyleSheet(f"""
            QLabel {{
                background: qlineargradient(x1:0, y1:0, x2:1, y2:0,
                    stop:0 {t['accent']}, stop:1 {QColor(t['accent']).lighter(130).name()});
                color: {'#101010' if self._is_light(t['accent']) else '#ffffff'};
                font-weight: bold;
                padding: 4px;
            }}
        """)
        self.info_label.setStyleSheet(f"""
            QLabel {{
                background: {t['dock_bg']};
                color: {t['dock_fg']};
                border: 1px solid {t['gutter_border']};
                padding: 6px;
                font-family: Segoe UI;
                font-size: 11px;
            }}
        """)
        self.code_label.setStyleSheet(f"font-family: Segoe UI; font-size: 11px; padding: 2px; color: {t['fg']};")
        self.code_edit.setStyleSheet(f"""
            QPlainTextEdit {{
                background: {t['bg']};
                border: 1px solid {t['gutter_border']};
                color: {t['fg']};
            }}
        """)
        self.apply_btn.setStyleSheet(f"""
            QPushButton {{
                background: {t['toolbar_bg']};
                color: {t['fg']};
                border: 1px solid {t['gutter_border']};
                padding: 4px 12px;
                font-family: Segoe UI;
            }}
            QPushButton:hover {{
                background: {t['sel']};
            }}
            QPushButton:pressed {{
                background: {t['accent']};
                color: #ffffff;
            }}
        """)

    def _is_light(self, color):
        c = QColor(color)
        yiq = (c.red() * 299 + c.green() * 587 + c.blue() * 114) / 1000
        return yiq >= 145

    def set_theme(self, theme):
        self.theme = theme
        self._apply_theme()

    def set_control(self, control):
        self.current_control = control
        if control is None:
            self.info_label.setText("No control selected")
            self.code_edit.setPlainText("")
            self.code_edit.setEnabled(False)
            self.apply_btn.setEnabled(False)
        else:
            self.info_label.setText(f"<b>{control.icon} {control.control_type}</b><br>"
                                   f"<small>{control.description}</small>")
            self.code_edit.setPlainText(control.code)
            self.code_edit.setEnabled(True)
            self.apply_btn.setEnabled(True)

    def _on_code_changed(self):
        pass

    def _apply_code(self):
        if self.current_control:
            new_code = self.code_edit.toPlainText()
            self.current_control.code = new_code
            self.current_control.update()
            self.code_changed.emit(self.current_control, new_code)


class VisualCanvasDock(QDockWidget):
    insertRequested = pyqtSignal(str)

    def __init__(self, theme, parent=None):
        super().__init__("Visual Canvas", parent)
        self.theme = theme
        self.setAllowedAreas(Qt.LeftDockWidgetArea | Qt.RightDockWidgetArea)
        self.setMinimumWidth(700)
        self._build_ui()

    def _build_ui(self):
        container = QWidget()
        main_layout = QVBoxLayout(container)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        toolbar = QWidget()
        tb_layout = QHBoxLayout(toolbar)
        tb_layout.setContentsMargins(4, 2, 4, 2)
        tb_layout.setSpacing(4)

        btn_insert = QPushButton("▶ Insert As Code")
        btn_insert.clicked.connect(self.insert_as_code)
        btn_clear = QPushButton("🗑 Clear Form")
        btn_clear.clicked.connect(self.clear_canvas)
        btn_reset = QPushButton("⟲ Reset View")
        btn_reset.clicked.connect(self._reset_view)

        tb_layout.addWidget(btn_insert)
        tb_layout.addWidget(btn_clear)
        tb_layout.addWidget(btn_reset)
        tb_layout.addStretch()
        self.toolbar = toolbar

        main_layout.addWidget(toolbar)

        content = QWidget()
        content_layout = QHBoxLayout(content)
        content_layout.setContentsMargins(0, 0, 0, 0)
        content_layout.setSpacing(0)

        self.toolbox = VBToolbox(self.theme)
        self.toolbox.setFixedWidth(160)
        self.toolbox.control_clicked.connect(self._add_control)
        content_layout.addWidget(self.toolbox)

        self.scene = VBFormScene(self.theme)
        self.view = VBFormView(self.scene)
        self.scene.control_edit_requested.connect(self._on_control_edit)
        self.scene.selectionChanged.connect(self._on_selection_changed)
        content_layout.addWidget(self.view, 1)

        self.properties = VBProperties(self.theme)
        self.properties.setFixedWidth(240)
        self.properties.code_changed.connect(self._on_code_changed)
        content_layout.addWidget(self.properties)

        main_layout.addWidget(content, 1)
        self.setWidget(container)
        self._apply_theme()

    def _apply_theme(self):
        t = self.theme
        self.toolbar.setStyleSheet(f"background: {t['toolbar_bg']}; border-bottom: 1px solid {t['gutter_border']};")
        btn_style = f"""
            QPushButton {{
                background: {t['toolbar_bg']};
                color: {t['fg']};
                border: 1px solid {t['gutter_border']};
                padding: 3px 10px;
                font-family: Segoe UI;
                font-size: 11px;
            }}
            QPushButton:hover {{ background: {t['sel']}; }}
            QPushButton:pressed {{ background: {t['accent']}; color: #ffffff; }}
        """
        for btn in self.toolbar.findChildren(QPushButton):
            btn.setStyleSheet(btn_style)

    def _add_control(self, data):
        ctrl_type, code, icon, desc = data
        if ctrl_type == "Custom":
            text, ok = QInputDialog.getMultiLineText(self, "Custom Control", "Vulpin code:", "")
            if not ok or not text.strip():
                return
            code = text.strip()

        center = self.view.mapToScene(self.view.viewport().rect().center())
        x = max(20, min(center.x() - 90, 440))
        y = max(40, min(center.y() - 40, 400))

        item = VBControlItem(ctrl_type, code, icon, desc, self.theme)
        item.setPos(x, y)
        self.scene.addItem(item)
        self.scene.clearSelection()
        item.setSelected(True)
        self.properties.set_control(item)

    def _on_control_edit(self, control):
        self.properties.set_control(control)
        self.view.setFocus()

    def _on_selection_changed(self):
        selected = self.scene.selectedItems()
        if selected and isinstance(selected[0], VBControlItem):
            self.properties.set_control(selected[0])
        else:
            self.properties.set_control(None)

    def _on_code_changed(self, control, new_code):
        pass

    def clear_canvas(self):
        self.scene.clear()
        self.properties.set_control(None)

    def _reset_view(self):
        self.view.resetTransform()
        self.view.centerOn(320, 240)

    def insert_as_code(self):
        items = [item for item in self.scene.items() if isinstance(item, VBControlItem)]
        if not items:
            QMessageBox.information(self, "Empty Form", "Add some controls to the form first.")
            return
        items.sort(key=lambda item: (item.pos().y(), item.pos().x()))
        blocks = []
        for item in items:
            code = item.code.strip()
            if code:
                blocks.append(code)
        if blocks:
            self.insertRequested.emit("\n\n".join(blocks) + "\n")

    def set_theme(self, theme):
        self.theme = theme
        self.toolbox.set_theme(theme)
        self.properties.set_theme(theme)
        self.scene.set_theme(theme)
        for item in self.scene.items():
            if isinstance(item, VBControlItem):
                item.set_theme(theme)
        self._apply_theme()


# ============== SETTINGS DIALOG ==============

class SettingsDialog(QDialog):
    def __init__(self, settings, themes, parent=None):
        super().__init__(parent)
        self.settings = settings.copy()
        self.themes = themes
        self.setWindowTitle("Settings")
        self.setMinimumSize(600, 500)
        self._build_ui()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        
        self.tabs = QTabWidget()
        
        # General tab
        general = QWidget()
        gen_layout = QFormLayout(general)
        
        self.auto_save_cb = QCheckBox("Enable auto-save")
        self.auto_save_cb.setChecked(self.settings.get("auto_save", False))
        gen_layout.addRow(self.auto_save_cb)
        
        self.auto_save_interval = QSpinBox()
        self.auto_save_interval.setRange(10, 3600)
        self.auto_save_interval.setValue(self.settings.get("auto_save_interval", 60))
        self.auto_save_interval.setSuffix(" seconds")
        gen_layout.addRow("Auto-save interval:", self.auto_save_interval)
        
        self.recent_limit = QSpinBox()
        self.recent_limit.setRange(1, 50)
        self.recent_limit.setValue(self.settings.get("recent_files_limit", 10))
        gen_layout.addRow("Recent files limit:", self.recent_limit)
        
        self.confirm_close_cb = QCheckBox("Confirm before closing unsaved files")
        self.confirm_close_cb.setChecked(self.settings.get("confirm_on_close", True))
        gen_layout.addRow(self.confirm_close_cb)
        
        self.restore_session_cb = QCheckBox("Restore last session on startup")
        self.restore_session_cb.setChecked(self.settings.get("restore_session", False))
        gen_layout.addRow(self.restore_session_cb)
        
        self.tabs.addTab(general, "General")
        
        # Editor tab
        editor = QWidget()
        ed_layout = QFormLayout(editor)
        
        self.tab_width = QSpinBox()
        self.tab_width.setRange(2, 8)
        self.tab_width.setValue(self.settings.get("tab_width", 4))
        ed_layout.addRow("Tab width:", self.tab_width)
        
        self.word_wrap_cb = QCheckBox("Enable word wrap")
        self.word_wrap_cb.setChecked(self.settings.get("word_wrap", False))
        ed_layout.addRow(self.word_wrap_cb)
        
        self.auto_close_cb = QCheckBox("Auto-close brackets and quotes")
        self.auto_close_cb.setChecked(self.settings.get("auto_close_brackets", True))
        ed_layout.addRow(self.auto_close_cb)
        
        self.auto_indent_cb = QCheckBox("Auto-indent on new line")
        self.auto_indent_cb.setChecked(self.settings.get("auto_indent", True))
        ed_layout.addRow(self.auto_indent_cb)
        
        self.autocomplete_cb = QCheckBox("Show autocomplete suggestions")
        self.autocomplete_cb.setChecked(self.settings.get("show_autocomplete", True))
        ed_layout.addRow(self.autocomplete_cb)
        
        self.tabs.addTab(editor, "Editor")
        
        # Appearance tab
        appearance = QWidget()
        app_layout = QFormLayout(appearance)
        
        self.theme_combo = QComboBox()
        self.theme_combo.addItems(list(self.themes.keys()))
        current_theme = self.settings.get("theme", "Dark (Catppuccin Mocha)")
        if current_theme in self.themes:
            self.theme_combo.setCurrentText(current_theme)
        app_layout.addRow("Theme:", self.theme_combo)
        
        self.font_family = QComboBox()
        fonts = ["JetBrains Mono", "Fira Code", "Source Code Pro", "Consolas", 
                 "Monaco", "Courier New", "DejaVu Sans Mono"]
        self.font_family.addItems(fonts)
        current_font = self.settings.get("font_family", "JetBrains Mono")
        idx = self.font_family.findText(current_font)
        if idx >= 0:
            self.font_family.setCurrentIndex(idx)
        app_layout.addRow("Font family:", self.font_family)
        
        self.font_size = QSpinBox()
        self.font_size.setRange(8, 36)
        self.font_size.setValue(self.settings.get("font_size", 12))
        app_layout.addRow("Font size:", self.font_size)
        
        self.line_numbers_cb = QCheckBox("Show line numbers")
        self.line_numbers_cb.setChecked(self.settings.get("show_line_numbers", True))
        app_layout.addRow(self.line_numbers_cb)
        
        self.output_height = QSpinBox()
        self.output_height.setRange(100, 600)
        self.output_height.setValue(self.settings.get("output_height", 220))
        self.output_height.setSuffix(" px")
        app_layout.addRow("Output panel height:", self.output_height)
        
        self.tabs.addTab(appearance, "Appearance")
        
        # Power tab
        power = QWidget()
        power_layout = QVBoxLayout(power)
        
        power_group = QGroupBox("Power Profile")
        pg_layout = QVBoxLayout(power_group)
        
        self.power_group = QButtonGroup(self)
        self.power_save_rb = QRadioButton("Power Save")
        self.power_save_rb.setToolTip(POWER_PROFILES["power_save"]["description"])
        self.balanced_rb = QRadioButton("Balanced")
        self.balanced_rb.setToolTip(POWER_PROFILES["balanced"]["description"])
        self.performance_rb = QRadioButton("Performance")
        self.performance_rb.setToolTip(POWER_PROFILES["performance"]["description"])
        
        self.power_group.addButton(self.power_save_rb, 0)
        self.power_group.addButton(self.balanced_rb, 1)
        self.power_group.addButton(self.performance_rb, 2)
        
        current_profile = self.settings.get("power_profile", "balanced")
        if current_profile == "power_save":
            self.power_save_rb.setChecked(True)
        elif current_profile == "performance":
            self.performance_rb.setChecked(True)
        else:
            self.balanced_rb.setChecked(True)
        
        pg_layout.addWidget(self.power_save_rb)
        pg_layout.addWidget(self.balanced_rb)
        pg_layout.addWidget(self.performance_rb)
        
        power_layout.addWidget(power_group)
        
        # Info labels
        info = QLabel(
            "<b>Power Save:</b> Slower algorithm refresh (1000ms) and autocomplete delay (500ms). "
            "Best for laptops on battery.<br><br>"
            "<b>Balanced:</b> Moderate refresh (400ms) and delay (250ms). "
            "Good for most users.<br><br>"
            "<b>Performance:</b> Fastest refresh (100ms) and delay (100ms). "
            "Uses more CPU but feels snappier."
        )
        info.setWordWrap(True)
        info.setStyleSheet("padding: 10px;")
        power_layout.addWidget(info)
        
        power_layout.addStretch()
        self.tabs.addTab(power, "Power")
        
        # Advanced tab
        advanced = QWidget()
        adv_layout = QFormLayout(advanced)
        
        self.vulpin_path = QLineEdit()
        self.vulpin_path.setPlaceholderText("Leave empty to auto-detect")
        self.vulpin_path.setText(self.settings.get("vulpin_path", ""))
        adv_layout.addRow("Vulpin executable:", self.vulpin_path)
        
        browse_btn = QPushButton("Browse...")
        browse_btn.clicked.connect(self._browse_vulpin)
        adv_layout.addRow("", browse_btn)
        
        self.terminal_shell = QLineEdit()
        self.terminal_shell.setPlaceholderText("Leave empty for system default")
        self.terminal_shell.setText(self.settings.get("terminal_shell", ""))
        adv_layout.addRow("Terminal shell:", self.terminal_shell)
        
        self.tabs.addTab(advanced, "Advanced")
        
        layout.addWidget(self.tabs)
        
        # Buttons
        buttons = QDialogButtonBox(
            QDialogButtonBox.Ok | QDialogButtonBox.Cancel | QDialogButtonBox.Apply
        )
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        buttons.button(QDialogButtonBox.Apply).clicked.connect(self._apply)
        layout.addWidget(buttons)

    def _browse_vulpin(self):
        path, _ = QFileDialog.getOpenFileName(self, "Select Vulpin Executable", "", "All Files (*)")
        if path:
            self.vulpin_path.setText(path)

    def _apply(self):
        self._collect_settings()

    def _collect_settings(self):
        self.settings["auto_save"] = self.auto_save_cb.isChecked()
        self.settings["auto_save_interval"] = self.auto_save_interval.value()
        self.settings["recent_files_limit"] = self.recent_limit.value()
        self.settings["confirm_on_close"] = self.confirm_close_cb.isChecked()
        self.settings["restore_session"] = self.restore_session_cb.isChecked()
        self.settings["tab_width"] = self.tab_width.value()
        self.settings["word_wrap"] = self.word_wrap_cb.isChecked()
        self.settings["auto_close_brackets"] = self.auto_close_cb.isChecked()
        self.settings["auto_indent"] = self.auto_indent_cb.isChecked()
        self.settings["show_autocomplete"] = self.autocomplete_cb.isChecked()
        self.settings["theme"] = self.theme_combo.currentText()
        self.settings["font_family"] = self.font_family.currentText()
        self.settings["font_size"] = self.font_size.value()
        self.settings["show_line_numbers"] = self.line_numbers_cb.isChecked()
        self.settings["output_height"] = self.output_height.value()
        
        profile_id = self.power_group.checkedId()
        if profile_id == 0:
            profile = "power_save"
        elif profile_id == 2:
            profile = "performance"
        else:
            profile = "balanced"
        self.settings["power_profile"] = profile
        self.settings["algo_refresh_ms"] = POWER_PROFILES[profile]["algo_refresh_ms"]
        self.settings["autocomplete_delay_ms"] = POWER_PROFILES[profile]["autocomplete_delay_ms"]
        
        self.settings["vulpin_path"] = self.vulpin_path.text().strip()
        self.settings["terminal_shell"] = self.terminal_shell.text().strip()

    def get_settings(self):
        self._collect_settings()
        return self.settings


class VulpinHighlighter(QSyntaxHighlighter):
    def __init__(self, document, theme):
        super().__init__(document)
        self.theme = theme
        self.rules = []
        self._build_rules()

    def _fmt(self, color_key, bold=False, italic=False):
        fmt = QTextCharFormat()
        fmt.setForeground(QColor(self.theme[color_key]))
        if bold:
            fmt.setFontWeight(QFont.Bold)
        if italic:
            fmt.setFontItalic(True)
        return fmt

    def _build_rules(self):
        self.rules = []
        self.rules.append((QRegularExpression(r'#.*$'), self._fmt("comment", italic=True)))
        self.rules.append((QRegularExpression(r'"[^"\\]*(?:\\.[^"\\]*)*"'), self._fmt("string")))
        self.rules.append((QRegularExpression(r"'[^'\\]*(?:\\.[^'\\]*)*'"), self._fmt("string")))
        self.rules.append((QRegularExpression(r'\$[a-zA-Z_]\w*'), self._fmt("variable", bold=True)))
        self.rules.append((QRegularExpression(r'\$[a-zA-Z_]\w*\s*\('), self._fmt("function", italic=True)))
        self.rules.append((QRegularExpression(r'\b\d+\.?\d*\b'), self._fmt("number")))
        cmd_chars = "GPASDKXQEULJFRTCYWVNZ!"
        self.rules.append((QRegularExpression(r'^\s*[' + cmd_chars + r']\b'), self._fmt("command", bold=True)))
        ctrl_chars = r'\?\:\;\@\&\~'
        self.rules.append((QRegularExpression(r'^\s*[' + ctrl_chars + r']'), self._fmt("control", bold=True)))
        self.rules.append((QRegularExpression(r'[+\-*/%=<>!&|^~]+'), self._fmt("operator")))
        self.rules.append((QRegularExpression(r'[()\[\]{}]'), self._fmt("bracket", bold=True)))
        self.rules.append((QRegularExpression(r'(?<![=<>!])=(?!=)'), self._fmt("operator", bold=True)))

    def set_theme(self, theme):
        self.theme = theme
        self._build_rules()
        self.rehighlight()

    def highlightBlock(self, text):
        for pattern, fmt in self.rules:
            it = pattern.globalMatch(text)
            while it.hasNext():
                match = it.next()
                self.setFormat(match.capturedStart(), match.capturedLength(), fmt)


class LineNumberArea(QWidget):
    def __init__(self, editor):
        super().__init__(editor)
        self.editor = editor

    def sizeHint(self):
        return QSize(self.editor.line_number_area_width(), 0)

    def paintEvent(self, event):
        self.editor.line_number_area_paint_event(event)


class CodeEditor(QPlainTextEdit):
    cursorMoved = pyqtSignal(int, int)

    def __init__(self, theme, settings, parent=None):
        super().__init__(parent)
        self.theme = theme
        self.settings = settings
        self._modified = False
        self._file_path = None
        self._programmatic_change = False
        self.font = QFont(settings.get("font_family", "JetBrains Mono"), settings.get("font_size", 12))
        self.font.setStyleHint(QFont.Monospace)
        self.setFont(self.font)
        self.setTabStopDistance(QFontMetrics(self.font).horizontalAdvance(" ") * settings.get("tab_width", 4))
        self.line_number_area = LineNumberArea(self)
        self.highlighter = VulpinHighlighter(self.document(), theme)
        self._setup_autocomplete()
        self.cursorPositionChanged.connect(self._on_cursor_changed)
        self.blockCountChanged.connect(self._update_line_number_area_width)
        self.updateRequest.connect(self._update_line_number_area)
        self.textChanged.connect(self._on_text_changed)
        self._update_line_number_area_width(0)
        self._apply_theme()

    def _apply_theme(self):
        t = self.theme
        self.setStyleSheet(f"""
            QPlainTextEdit {{ background-color: {t['bg']}; color: {t['fg']}; selection-background-color: {t['sel']}; selection-color: {t['fg']}; border: none; }}
            QPlainTextEdit QScrollBar:vertical {{ background: {t['bg']}; width: 10px; margin: 0; }}
            QPlainTextEdit QScrollBar::handle:vertical {{ background: {t['scrollbar']}; min-height: 30px; border-radius: 5px; }}
            QPlainTextEdit QScrollBar::handle:vertical:hover {{ background: {t['scrollbar_hover']}; }}
            QPlainTextEdit QScrollBar::add-line:vertical, QPlainTextEdit QScrollBar::sub-line:vertical {{ height: 0; }}
        """)
        pal = self.palette()
        pal.setColor(QPalette.Text, QColor(t['fg']))
        pal.setColor(QPalette.Base, QColor(t['bg']))
        self.setPalette(pal)
        self.highlighter.set_theme(t)
        self.viewport().update()
        self.line_number_area.update()

    def set_theme(self, theme):
        self.theme = theme
        self._apply_theme()

    def apply_settings(self, settings):
        self.settings = settings
        self.font = QFont(settings.get("font_family", "JetBrains Mono"), settings.get("font_size", 12))
        self.font.setStyleHint(QFont.Monospace)
        self.setFont(self.font)
        self.setTabStopDistance(QFontMetrics(self.font).horizontalAdvance(" ") * settings.get("tab_width", 4))
        self._update_line_number_area_width(0)

    def line_number_area_width(self):
        if not self.settings.get("show_line_numbers", True):
            return 0
        digits = max(1, len(str(self.blockCount())))
        return 15 + QFontMetrics(self.font).horizontalAdvance("9") * digits

    def _update_line_number_area_width(self, _):
        self.setViewportMargins(self.line_number_area_width(), 0, 0, 0)

    def _update_line_number_area(self, rect, dy):
        if dy:
            self.line_number_area.scroll(0, dy)
        else:
            self.line_number_area.update(0, rect.y(), self.line_number_area.width(), rect.height())
        if rect.contains(self.viewport().rect()):
            self._update_line_number_area_width(0)

    def resizeEvent(self, event):
        super().resizeEvent(event)
        cr = self.contentsRect()
        self.line_number_area.setGeometry(QRect(cr.left(), cr.top(), self.line_number_area_width(), cr.height()))

    def line_number_area_paint_event(self, event):
        if not self.settings.get("show_line_numbers", True):
            return
        painter = QPainter(self.line_number_area)
        t = self.theme
        painter.fillRect(event.rect(), QColor(t['gutter_bg']))
        painter.setPen(QColor(t['gutter_border']))
        painter.drawLine(event.rect().topRight(), event.rect().bottomRight())
        block = self.firstVisibleBlock()
        block_number = block.blockNumber()
        top = int(self.blockBoundingGeometry(block).translated(self.contentOffset()).top())
        bottom = top + int(self.blockBoundingRect(block).height())
        current_block = self.textCursor().blockNumber()
        while block.isValid() and top <= event.rect().bottom():
            if block.isVisible() and bottom >= event.rect().top():
                number = str(block_number + 1)
                if block_number == current_block:
                    painter.setPen(QColor(t['line_hl']))
                    font = painter.font()
                    font.setBold(True)
                    painter.setFont(font)
                else:
                    painter.setPen(QColor(t['line_fg']))
                    font = painter.font()
                    font.setBold(False)
                    painter.setFont(font)
                painter.drawText(0, top, self.line_number_area.width() - 8, QFontMetrics(self.font).height(), Qt.AlignRight, number)
            block = block.next()
            top = bottom
            bottom = top + int(self.blockBoundingRect(block).height())
            block_number += 1
        painter.end()

    def _on_cursor_changed(self):
        selections = []
        if not self.isReadOnly():
            sel = QTextEdit.ExtraSelection()
            sel.format.setBackground(QColor(self.theme['current_line']))
            sel.format.setProperty(QTextFormat.FullWidthSelection, True)
            sel.cursor = self.textCursor()
            sel.cursor.clearSelection()
            selections.append(sel)
        self._add_bracket_selections(selections)
        self.setExtraSelections(selections)
        cursor = self.textCursor()
        self.cursorMoved.emit(cursor.blockNumber() + 1, cursor.columnNumber() + 1)

    BRACKET_PAIRS = {'(': ')', '[': ']', '{': '}', ')': '(', ']': '[', '}': '{'}

    def _add_bracket_selections(self, selections):
        cursor = self.textCursor()
        pos = cursor.position()
        text = self.toPlainText()
        if pos <= 0 or pos >= len(text):
            return
        char_before = text[pos - 1] if pos > 0 else ''
        char_at = text[pos] if pos < len(text) else ''
        bracket_char = None
        bracket_pos = -1
        if char_before in self.BRACKET_PAIRS:
            bracket_char = char_before
            bracket_pos = pos - 1
        elif char_at in self.BRACKET_PAIRS:
            bracket_char = char_at
            bracket_pos = pos
        if bracket_char is None:
            return
        match_pos = self._find_matching_bracket(text, bracket_pos, bracket_char)
        if match_pos >= 0:
            sel1 = QTextEdit.ExtraSelection()
            sel1.format.setBackground(QColor(self.theme['match_bracket']))
            sel1.format.setForeground(QColor(self.theme['bg']))
            sel1.cursor = QTextCursor(self.document())
            sel1.cursor.setPosition(bracket_pos)
            sel1.cursor.movePosition(QTextCursor.Right, QTextCursor.KeepAnchor)
            sel2 = QTextEdit.ExtraSelection()
            sel2.format.setBackground(QColor(self.theme['match_bracket']))
            sel2.format.setForeground(QColor(self.theme['bg']))
            sel2.cursor = QTextCursor(self.document())
            sel2.cursor.setPosition(match_pos)
            sel2.cursor.movePosition(QTextCursor.Right, QTextCursor.KeepAnchor)
            selections.append(sel1)
            selections.append(sel2)

    def _find_matching_bracket(self, text, pos, char):
        pair = self.BRACKET_PAIRS[char]
        direction = 1 if char in '([{' else -1
        depth = 0
        i = pos
        while 0 <= i < len(text):
            if text[i] == char:
                depth += 1
            elif text[i] == pair:
                depth -= 1
                if depth == 0:
                    return i
            i += direction
        return -1

    def _setup_autocomplete(self):
        words = []
        for cmd, data in VULPIN_COMMANDS.items():
            words.append(f"{cmd}  — {data[0]}")
        for ctrl, data in VULPIN_CONTROL.items():
            words.append(f"{ctrl}  — {data[0]}")
        for snip in VULPIN_SNIPPETS:
            words.append(snip)
        words.extend(["if", "while", "for", "func", "try", "switch", "print", "input", "import", "label", "jump"])
        self._completer = QCompleter(words, self)
        self._completer.setWidget(self)
        self._completer.setCompletionMode(QCompleter.PopupCompletion)
        self._completer.setCaseSensitivity(Qt.CaseInsensitive)
        self._completer.setFilterMode(Qt.MatchContains)
        self._completer.activated.connect(self._insert_completion)
        self._completer.popup().setFont(self.font)

    def _insert_completion(self, completion):
        clean = completion.split("  —")[0].strip()
        if clean in VULPIN_SNIPPETS:
            cursor = self.textCursor()
            cursor.movePosition(QTextCursor.Left, QTextCursor.KeepAnchor, len(clean))
            cursor.removeSelectedText()
            cursor.insertText(VULPIN_SNIPPETS[clean])
            return
        cursor = self.textCursor()
        cursor.movePosition(QTextCursor.Left, QTextCursor.KeepAnchor, len(self._completer.completionPrefix()))
        cursor.removeSelectedText()
        cursor.insertText(clean + " ")
        self.setTextCursor(cursor)

    def _on_text_changed(self):
        if self._programmatic_change:
            return
        if not self.settings.get("show_autocomplete", True):
            return
        self._modified = True
        cursor = self.textCursor()
        cursor.select(QTextCursor.WordUnderCursor)
        prefix = cursor.selectedText().strip()
        if len(prefix) >= 1 and prefix.isalnum():
            self._completer.setCompletionPrefix(prefix)
            popup = self._completer.popup()
            model = self._completer.completionModel()
            if model.rowCount() > 0:
                popup.setCurrentIndex(model.index(0, 0))
                cr = self.cursorRect()
                cr.setWidth(popup.sizeHintForColumn(0) + 40)
                self._completer.complete(cr)
            else:
                popup.hide()
        else:
            self._completer.popup().hide()

    def keyPressEvent(self, event):
        key = event.key()
        modifiers = event.modifiers()
        cursor = self.textCursor()
        tab_width = self.settings.get("tab_width", 4)
        tab_str = " " * tab_width
        
        if key == Qt.Key_Tab:
            if cursor.hasSelection():
                self._indent_selection(True)
            else:
                cursor.insertText(tab_str)
            return
        if key == Qt.Key_Backtab:
            self._indent_selection(False)
            return
        if key in (Qt.Key_Return, Qt.Key_Enter):
            if self.settings.get("auto_indent", True):
                self._auto_indent()
            else:
                super().keyPressEvent(event)
            return
        
        if self.settings.get("auto_close_brackets", True):
            auto_pairs = {'(': ')', '[': ']', '{': '}', '"': '"', "'": "'"}
            if event.text() in auto_pairs and not (modifiers & Qt.ControlModifier):
                char = event.text()
                close = auto_pairs[char]
                if cursor.hasSelection():
                    selected = cursor.selectedText()
                    cursor.insertText(char + selected + close)
                else:
                    cursor.insertText(char + close)
                    cursor.movePosition(QTextCursor.Left)
                    self.setTextCursor(cursor)
                return
            if key == Qt.Key_Backspace and not cursor.hasSelection():
                pos = cursor.position()
                text = self.toPlainText()
                if pos > 0 and pos < len(text):
                    before = text[pos - 1]
                    after = text[pos]
                    if (before, after) in [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ("'", "'")]:
                        cursor.deletePreviousChar()
                        cursor.deleteChar()
                        self.setTextCursor(cursor)
                        return
        super().keyPressEvent(event)

    def _auto_indent(self):
        cursor = self.textCursor()
        block = cursor.block()
        text = block.text()
        indent = ""
        for ch in text:
            if ch in (' ', '\t'):
                indent += ch
            else:
                break
        stripped = text.strip()
        extra_indent = ""
        if stripped:
            first_char = stripped[0]
            if first_char in BLOCK_OPENERS or stripped.startswith("F ") or stripped.startswith("O ") or stripped.startswith("@ "):
                extra_indent = " " * self.settings.get("tab_width", 4)
        cursor.insertText("\n" + indent + extra_indent)
        self.setTextCursor(cursor)

    def _indent_selection(self, indent=True):
        cursor = self.textCursor()
        start = cursor.selectionStart()
        end = cursor.selectionEnd()
        cursor.setPosition(start)
        cursor.movePosition(QTextCursor.StartOfBlock)
        cursor.setPosition(end, QTextCursor.KeepAnchor)
        cursor.movePosition(QTextCursor.EndOfBlock, QTextCursor.KeepAnchor)
        selected = cursor.selectedText()
        lines = selected.split("\u2029")
        tab_str = " " * self.settings.get("tab_width", 4)
        if indent:
            new_lines = [tab_str + line for line in lines]
        else:
            new_lines = []
            for line in lines:
                if line.startswith(tab_str):
                    new_lines.append(line[len(tab_str):])
                elif line.startswith("\t"):
                    new_lines.append(line[1:])
                else:
                    new_lines.append(line)
        cursor.insertText("\n".join(new_lines))

    @property
    def file_path(self):
        return self._file_path

    @file_path.setter
    def file_path(self, path):
        self._file_path = path

    @property
    def is_modified(self):
        return self._modified

    @is_modified.setter
    def is_modified(self, val):
        self._modified = val

    def set_font_size(self, size):
        self.font.setPointSize(size)
        self.setFont(self.font)
        self.setTabStopDistance(QFontMetrics(self.font).horizontalAdvance(" ") * self.settings.get("tab_width", 4))
        self._update_line_number_area_width(0)


class SearchReplaceDialog(QDialog):
    def __init__(self, editor, parent=None):
        super().__init__(parent)
        self.editor = editor
        self.setWindowTitle("Search & Replace")
        self.setMinimumWidth(420)
        self._build_ui()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        h1 = QHBoxLayout()
        h1.addWidget(QLabel("Find:"))
        self.search_input = QLineEdit()
        self.search_input.setPlaceholderText("Search text...")
        self.search_input.returnPressed.connect(self.find_next)
        h1.addWidget(self.search_input)
        layout.addLayout(h1)
        h2 = QHBoxLayout()
        h2.addWidget(QLabel("Replace:"))
        self.replace_input = QLineEdit()
        self.replace_input.setPlaceholderText("Replace with...")
        h2.addWidget(self.replace_input)
        layout.addLayout(h2)
        h3 = QHBoxLayout()
        self.case_cb = QCheckBox("Case sensitive")
        self.regex_cb = QCheckBox("Regex")
        h3.addWidget(self.case_cb)
        h3.addWidget(self.regex_cb)
        h3.addStretch()
        layout.addLayout(h3)
        h4 = QHBoxLayout()
        self.btn_find = QPushButton("Find Next")
        self.btn_find.clicked.connect(self.find_next)
        self.btn_replace = QPushButton("Replace")
        self.btn_replace.clicked.connect(self.replace_one)
        self.btn_replace_all = QPushButton("Replace All")
        self.btn_replace_all.clicked.connect(self.replace_all)
        h4.addWidget(self.btn_find)
        h4.addWidget(self.btn_replace)
        h4.addWidget(self.btn_replace_all)
        h4.addStretch()
        layout.addLayout(h4)
        self.status_label = QLabel("")
        layout.addWidget(self.status_label)

    def find_next(self):
        text = self.search_input.text()
        if not text:
            return
        flags = QTextDocument.FindFlags()
        if self.case_cb.isChecked():
            flags |= QTextDocument.FindCaseSensitively
        if self.regex_cb.isChecked():
            cursor = self.editor.textCursor()
            doc = self.editor.document()
            regex = QRegularExpression(text)
            if not self.case_cb.isChecked():
                regex.setPatternOptions(QRegularExpression.CaseInsensitiveOption)
            result = doc.find(regex, cursor)
            if not result.isNull():
                self.editor.setTextCursor(result)
                self.status_label.setText("Found.")
            else:
                cursor.movePosition(QTextCursor.Start)
                result = doc.find(regex, cursor)
                if not result.isNull():
                    self.editor.setTextCursor(result)
                    self.status_label.setText("Wrapped around. Found.")
                else:
                    self.status_label.setText("Not found.")
        else:
            found = self.editor.find(text, flags)
            if not found:
                cursor = self.editor.textCursor()
                cursor.movePosition(QTextCursor.Start)
                self.editor.setTextCursor(cursor)
                found = self.editor.find(text, flags)
                self.status_label.setText("Wrapped around. Found." if found else "Not found.")
            else:
                self.status_label.setText("Found.")

    def replace_one(self):
        cursor = self.editor.textCursor()
        if cursor.hasSelection():
            cursor.insertText(self.replace_input.text())
        self.find_next()

    def replace_all(self):
        text = self.search_input.text()
        if not text:
            return
        doc = self.editor.document()
        edit_cursor = QTextCursor(doc)
        edit_cursor.beginEditBlock()
        count = 0
        if self.regex_cb.isChecked():
            regex = QRegularExpression(text)
            if not self.case_cb.isChecked():
                regex.setPatternOptions(QRegularExpression.CaseInsensitiveOption)
            full_text = doc.toPlainText()
            it = regex.globalMatch(full_text)
            replacements = []
            while it.hasNext():
                m = it.next()
                replacements.append((m.capturedStart(), m.capturedEnd()))
            for start, end in reversed(replacements):
                c = QTextCursor(doc)
                c.setPosition(start)
                c.setPosition(end, QTextCursor.KeepAnchor)
                c.insertText(self.replace_input.text())
                count += 1
        else:
            flags = QTextDocument.FindFlags()
            if self.case_cb.isChecked():
                flags |= QTextDocument.FindCaseSensitively
            cur = QTextCursor(doc)
            while True:
                cur = doc.find(text, cur, flags)
                if cur.isNull():
                    break
                cur.insertText(self.replace_input.text())
                count += 1
        edit_cursor.endEditBlock()
        self.status_label.setText(f"Replaced {count} occurrence(s).")


class ReferenceDock(QDockWidget):
    insertRequested = pyqtSignal(str)

    def __init__(self, theme, parent=None):
        super().__init__("Reference", parent)
        self.theme = theme
        self.setAllowedAreas(Qt.LeftDockWidgetArea | Qt.RightDockWidgetArea)
        self.setMinimumWidth(260)
        self._build_ui()

    def _build_ui(self):
        self.tree = QTreeWidget()
        self.tree.setHeaderLabels(["Command", "Description"])
        self.tree.setColumnWidth(0, 80)
        self.tree.itemDoubleClicked.connect(self._on_item_double_clicked)
        cmd_group = QTreeWidgetItem(["Commands"])
        cmd_group.setExpanded(True)
        for char, data in sorted(VULPIN_COMMANDS.items()):
            item = QTreeWidgetItem([char, f"{data[0]} — {data[1]}"])
            item.setData(0, Qt.UserRole, char)
            cmd_group.addChild(item)
        self.tree.addTopLevelItem(cmd_group)
        ctrl_group = QTreeWidgetItem(["Control Flow"])
        ctrl_group.setExpanded(True)
        for char, data in sorted(VULPIN_CONTROL.items()):
            item = QTreeWidgetItem([char, f"{data[0]} — {data[1]}"])
            item.setData(0, Qt.UserRole, char)
            ctrl_group.addChild(item)
        self.tree.addTopLevelItem(ctrl_group)
        snip_group = QTreeWidgetItem(["Snippets"])
        snip_group.setExpanded(True)
        for name, code in sorted(VULPIN_SNIPPETS.items()):
            item = QTreeWidgetItem([name, code.split('\n')[0] + "..."])
            item.setData(0, Qt.UserRole, f"__snippet__{name}")
            snip_group.addChild(item)
        self.tree.addTopLevelItem(snip_group)
        self.setWidget(self.tree)

    def _on_item_double_clicked(self, item, column):
        data = item.data(0, Qt.UserRole)
        if data:
            if data.startswith("__snippet__"):
                self.insertRequested.emit(VULPIN_SNIPPETS[data[len("__snippet__"):]])
            else:
                self.insertRequested.emit(data + " ")

    def set_theme(self, theme):
        self.theme = theme


class AlgorithmDock(QDockWidget):
    refreshRequested = pyqtSignal()

    def __init__(self, theme, parent=None):
        super().__init__("Algorithm Viewer", parent)
        self.theme = theme
        self.last_text = ""
        self.setAllowedAreas(Qt.LeftDockWidgetArea | Qt.RightDockWidgetArea)
        self.setMinimumWidth(360)
        self._build_ui()
        self._apply_theme()

    def _build_ui(self):
        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(2)
        self.refresh_btn = QPushButton("Refresh Algorithm")
        self.refresh_btn.clicked.connect(self.refreshRequested.emit)
        self.tree = QListWidget()
        self.tree.setSpacing(6)
        layout.addWidget(self.refresh_btn)
        layout.addWidget(self.tree)
        self.setWidget(container)

    def _apply_theme(self):
        self.tree.setStyleSheet(f"""
            QListWidget {{
                background-color: {self.theme['dock_bg']};
                color: {self.theme['dock_fg']};
                border: none;
                outline: none;
            }}
            QListWidget::item {{
                background: transparent;
                border: none;
            }}
        """)

    def _line_color(self, s):
        t = self.theme
        if not s:
            return t.get("fg")
        first = s[0]
        if first in ("?", ":", ";", "@", "&", "~"):
            return t.get("control")
        if first in ("F", "R"):
            return t.get("function")
        if first in ("T", "C", "Y"):
            return t.get("builtin")
        if first in ("W", "V", "N", "Z"):
            return t.get("keyword")
        if first in ("G", "P"):
            return t.get("string")
        if first == "K":
            return t.get("variable")
        if first in ("A", "S") or ("=" in s and not s.startswith("=")):
            return t.get("variable")
        if first == "O":
            return t.get("control")
        if first in ("D", "U", "X", "Q"):
            return t.get("operator")
        if first == "E":
            return t.get("output_err")
        if first == "!":
            return t.get("operator")
        return t.get("fg")

    def refresh(self, text):
        if text == self.last_text:
            return
        self.last_text = text
        self.tree.clear()
        lines = text.splitlines()
        for idx, raw in enumerate(lines, 1):
            s = raw.strip()
            if not s or s.startswith("#"):
                continue
            color = self._line_color(s)
            card = QWidget()
            card_layout = QHBoxLayout(card)
            card_layout.setContentsMargins(8, 6, 8, 6)
            card_layout.setSpacing(10)
            number_label = QLabel(str(idx))
            number_label.setFixedWidth(34)
            number_label.setAlignment(Qt.AlignRight | Qt.AlignVCenter)
            number_label.setStyleSheet(f"color: {self.theme.get('accent', '#89b4fa')}; background: transparent; font-weight: bold;")
            code_label = QLabel(s)
            code_label.setTextFormat(Qt.PlainText)
            code_label.setStyleSheet(f"color: {color}; background: transparent; font-family: 'JetBrains Mono';")
            card_layout.addWidget(number_label)
            card_layout.addWidget(code_label, 1)
            card.setStyleSheet(f"""
                QWidget {{
                    background-color: {self.theme['toolbar_bg']};
                    border: 1px solid {self.theme['gutter_border']};
                    border-radius: 6px;
                }}
            """)
            item = QListWidgetItem()
            item.setSizeHint(card.sizeHint())
            self.tree.addItem(item)
            self.tree.setItemWidget(item, card)

    def set_theme(self, theme):
        self.theme = theme
        self._apply_theme()
        self.last_text = ""
        self.refresh(self.last_text)


class OutputConsole(QWidget):
    inputSubmitted = pyqtSignal(str)

    def __init__(self, theme, parent=None):
        super().__init__(parent)
        self.theme = theme
        self._build_ui()
        self._apply_theme()
        self.set_input_enabled(False)

    def _build_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self.output = QTextEdit()
        self.output.setReadOnly(True)
        self.output.setFont(QFont("JetBrains Mono", 11))

        input_bar = QWidget()
        input_layout = QHBoxLayout(input_bar)
        input_layout.setContentsMargins(6, 4, 6, 4)
        input_layout.setSpacing(6)

        self.prompt_label = QLabel("❯")
        self.prompt_label.setFixedWidth(14)
        self.prompt_label.setAlignment(Qt.AlignCenter)

        self.input = QLineEdit()
        self.input.setPlaceholderText("Type input for the running script and press Enter...")
        self.input.setFont(QFont("JetBrains Mono", 11))
        self.input.returnPressed.connect(self._on_input_submitted)

        self.send_btn = QPushButton("Send")
        self.send_btn.setFixedWidth(60)
        self.send_btn.clicked.connect(self._on_input_submitted)

        input_layout.addWidget(self.prompt_label)
        input_layout.addWidget(self.input, 1)
        input_layout.addWidget(self.send_btn)

        layout.addWidget(self.output, 1)
        layout.addWidget(input_bar)

    def _apply_theme(self):
        t = self.theme
        self.output.setStyleSheet(f"""
            QTextEdit {{
                background-color: {t['output_bg']};
                color: {t['output_fg']};
                border: none;
                border-top: 1px solid {t['gutter_border']};
            }}
        """)
        input_bar_style = f"""
            QWidget {{
                background-color: {t['toolbar_bg']};
                border-top: 1px solid {t['gutter_border']};
            }}
            QLabel {{
                color: {t.get('accent', '#89b4fa')};
                font-weight: bold;
                font-family: 'JetBrains Mono';
            }}
            QLineEdit {{
                background-color: {t['bg']};
                color: {t['fg']};
                border: 1px solid {t['gutter_border']};
                border-radius: 3px;
                padding: 4px 8px;
            }}
            QLineEdit:focus {{
                border-color: {t.get('accent', '#89b4fa')};
            }}
            QPushButton {{
                background-color: {t.get('accent', '#89b4fa')};
                color: #ffffff;
                border: none;
                border-radius: 3px;
                padding: 4px 10px;
                font-weight: bold;
            }}
            QPushButton:hover {{
                background-color: {t.get('accent', '#89b4fa')};
                opacity: 0.9;
            }}
            QPushButton:disabled {{
                background-color: {t['gutter_border']};
                color: {t['line_fg']};
            }}
        """
        for child in self.children():
            if isinstance(child, QWidget) and child is not self.output:
                child.setStyleSheet(input_bar_style)

    def set_theme(self, theme):
        self.theme = theme
        self._apply_theme()

    def set_input_enabled(self, enabled):
        self.input.setEnabled(enabled)
        self.send_btn.setEnabled(enabled)
        if enabled:
            self.input.setPlaceholderText("Type input and press Enter...")
            self.prompt_label.setStyleSheet(f"color: {self.theme.get('accent', '#89b4fa')}; font-weight: bold;")
        else:
            self.input.setPlaceholderText("Start a script to enable input...")
            self.prompt_label.setStyleSheet(f"color: {self.theme['line_fg']}; font-weight: bold;")

    def _on_input_submitted(self):
        text = self.input.text()
        if not self.input.isEnabled():
            return
        self.inputSubmitted.emit(text)
        self.write(f"> {text}", "accent")
        self.input.clear()
        self.input.setFocus()

    def write(self, text, color_key="output_fg"):
        color = self.theme.get(color_key, self.theme["output_fg"])
        self.output.setTextColor(QColor(color))
        self.output.append(text)
        self.output.verticalScrollBar().setValue(self.output.verticalScrollBar().maximum())

    def write_ok(self, text):
        self.write(text, "output_ok")

    def write_err(self, text):
        self.write(text, "output_err")

    def clear_output(self):
        self.output.clear()


class TerminalInput(QLineEdit):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.history = []
        self.index = -1

    def add_history(self, cmd):
        if cmd and (not self.history or self.history[-1] != cmd):
            self.history.append(cmd)
        self.index = len(self.history)

    def keyPressEvent(self, event):
        if event.key() == Qt.Key_Up:
            if self.history:
                if self.index == -1 or self.index >= len(self.history):
                    self.index = len(self.history) - 1
                elif self.index > 0:
                    self.index -= 1
                self.setText(self.history[self.index])
            return
        if event.key() == Qt.Key_Down:
            if self.history:
                if self.index < len(self.history) - 1:
                    self.index += 1
                    self.setText(self.history[self.index])
                else:
                    self.index = len(self.history)
                    self.setText("")
            return
        super().keyPressEvent(event)


class TerminalDock(QDockWidget):
    def __init__(self, theme, settings, parent=None):
        super().__init__("Terminal", parent)
        self.theme = theme
        self.settings = settings
        self.cwd = os.path.expanduser("~")
        self.setAllowedAreas(Qt.AllDockWidgetAreas)
        self.setMinimumHeight(180)
        self._build_ui()

    def _build_ui(self):
        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(4)
        top = QWidget()
        top_layout = QHBoxLayout(top)
        top_layout.setContentsMargins(6, 4, 6, 0)
        top_layout.setSpacing(6)
        self.prompt_label = QLabel(self.cwd)
        self.prompt_label.setMinimumWidth(200)
        clear_btn = QPushButton("Clear")
        clear_btn.clicked.connect(self.clear_output)
        top_layout.addWidget(self.prompt_label, 1)
        top_layout.addWidget(clear_btn)
        self.output = QTextEdit()
        self.output.setReadOnly(True)
        self.output.setFont(QFont("JetBrains Mono", 11))
        self.input = TerminalInput()
        self.input.setPlaceholderText("Enter command...")
        self.input.returnPressed.connect(self.run_command)
        layout.addWidget(top)
        layout.addWidget(self.output)
        layout.addWidget(self.input)
        self.setWidget(container)

    def clear_output(self):
        self.output.clear()

    def _append(self, text, color=None):
        if color:
            self.output.setTextColor(QColor(color))
        else:
            self.output.setTextColor(QColor(self.theme["fg"]))
        self.output.append(text)
        self.output.verticalScrollBar().setValue(self.output.verticalScrollBar().maximum())

    def run_command(self):
        cmd = self.input.text().strip()
        if not cmd:
            return
        self.input.add_history(cmd)
        self.input.clear()
        self._append(f"$ {cmd}", self.theme.get("accent", "#89b4fa"))
        if cmd == "clear":
            self.clear_output()
            return
        if cmd == "cd":
            self.cwd = os.path.expanduser("~")
            os.chdir(self.cwd)
            self.prompt_label.setText(self.cwd)
            self._append(self.cwd)
            return
        if cmd.startswith("cd "):
            path = cmd[3:].strip().strip('"').strip("'")
            path = os.path.abspath(os.path.expanduser(path))
            if os.path.isdir(path):
                self.cwd = path
                os.chdir(self.cwd)
                self.prompt_label.setText(self.cwd)
                self._append(self.cwd)
            else:
                self._append(f"Not a directory: {path}", self.theme["output_err"])
            return
        
        shell = self.settings.get("terminal_shell", "") or None
        try:
            result = subprocess.run(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, 
                                   universal_newlines=True, cwd=self.cwd, timeout=30, executable=shell)
            if result.stdout:
                self._append(result.stdout.rstrip())
            if result.stderr:
                self._append(result.stderr.rstrip(), self.theme["output_err"])
        except subprocess.TimeoutExpired:
            self._append("Command timed out after 30 seconds", self.theme["output_err"])
        except Exception as e:
            self._append(str(e), self.theme["output_err"])

    def set_theme(self, theme):
        self.theme = theme


class VulpinIDE(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Vulpin IDE")
        self.setMinimumSize(1150, 720)
        self.resize(1450, 880)
        self.settings_path = os.path.join(os.path.expanduser("~"), ".vulpin_ide_settings.json")
        self.settings = self._load_settings()
        self.current_theme_name = self.settings.get("theme", "Dark (Catppuccin Mocha)")
        if self.current_theme_name not in THEMES:
            self.current_theme_name = "Dark (Catppuccin Mocha)"
        self.theme = THEMES[self.current_theme_name]
        self.recent_files = self.settings.get("recent_files", [])
        self._algo_timer = QTimer()
        self._algo_timer.setSingleShot(True)
        self._algo_timer.setInterval(self.settings.get("algo_refresh_ms", 400))
        self._algo_timer.timeout.connect(self._refresh_algorithm)
        self._last_algo_text = ""
        self._build_ui()
        self._build_dock()
        self._build_menu()
        self._build_toolbar()
        self._build_statusbar()
        self._apply_global_theme()
        if len(sys.argv) > 1 and os.path.isfile(sys.argv[1]):
            self.open_file(sys.argv[1])
        else:
            self.new_file()

    def _load_settings(self):
        try:
            if os.path.exists(self.settings_path):
                with open(self.settings_path, "r") as f:
                    saved = json.load(f)
                    merged = DEFAULT_SETTINGS.copy()
                    merged.update(saved)
                    return merged
        except Exception:
            pass
        return DEFAULT_SETTINGS.copy()

    def _save_settings(self):
        try:
            with open(self.settings_path, "w") as f:
                json.dump(self.settings, f, indent=2)
        except Exception:
            pass

    def _build_ui(self):
        central = QWidget()
        layout = QVBoxLayout(central)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        self.splitter = QSplitter(Qt.Vertical)
        self.tabs = QTabWidget()
        self.tabs.setTabsClosable(True)
        self.tabs.setMovable(True)
        self.tabs.setDocumentMode(False)
        self.tabs.tabCloseRequested.connect(self.close_tab)
        self.tabs.currentChanged.connect(self._on_tab_changed)
        self.output = OutputConsole(self.theme)
        self.output.inputSubmitted.connect(self._send_input_to_process)
        self.output.setMaximumHeight(self.settings.get("output_height", 220))
        self.splitter.addWidget(self.tabs)
        self.splitter.addWidget(self.output)
        self.splitter.setStretchFactor(0, 4)
        self.splitter.setStretchFactor(1, 1)
        layout.addWidget(self.splitter)
        self.setCentralWidget(central)

    def _build_dock(self):
        self.setDockOptions(QMainWindow.AllowTabbedDocks | QMainWindow.AllowNestedDocks | QMainWindow.AnimatedDocks)
        self.setTabPosition(Qt.RightDockWidgetArea, QTabWidget.South)
        self.setTabPosition(Qt.LeftDockWidgetArea, QTabWidget.South)
        self.ref_dock = ReferenceDock(self.theme, self)
        self.blueprints_dock = BlueprintsDock(self.theme, self)
        self.algorithm_dock = AlgorithmDock(self.theme, self)
        self.visual_dock = VisualCanvasDock(self.theme, self)
        self.terminal_dock = TerminalDock(self.theme, self.settings, self)
        for dock in (self.ref_dock, self.blueprints_dock, self.algorithm_dock, self.visual_dock, self.terminal_dock):
            dock.setFeatures(QDockWidget.DockWidgetMovable | QDockWidget.DockWidgetClosable)
        self.ref_dock.insertRequested.connect(self._insert_from_dock)
        self.blueprints_dock.insertRequested.connect(self._insert_from_dock)
        self.blueprints_dock.syncCodeRequested.connect(self._sync_blueprint_code)
        self.visual_dock.insertRequested.connect(self._insert_from_dock)
        self.algorithm_dock.refreshRequested.connect(self._refresh_algorithm)
        self.addDockWidget(Qt.RightDockWidgetArea, self.ref_dock)
        self.addDockWidget(Qt.RightDockWidgetArea, self.blueprints_dock)
        self.addDockWidget(Qt.RightDockWidgetArea, self.algorithm_dock)
        self.addDockWidget(Qt.RightDockWidgetArea, self.visual_dock)
        self.tabifyDockWidget(self.ref_dock, self.blueprints_dock)
        self.tabifyDockWidget(self.blueprints_dock, self.algorithm_dock)
        self.tabifyDockWidget(self.algorithm_dock, self.visual_dock)
        self.addDockWidget(Qt.BottomDockWidgetArea, self.terminal_dock)
        self.blueprints_dock.raise_()

    def _build_menu(self):
        menubar = self.menuBar()
        file_menu = menubar.addMenu("&File")
        act_new = QAction("&New", self)
        act_new.setShortcut(QKeySequence.New)
        act_new.triggered.connect(self.new_file)
        file_menu.addAction(act_new)
        act_open = QAction("&Open...", self)
        act_open.setShortcut(QKeySequence.Open)
        act_open.triggered.connect(self.open_file_dialog)
        file_menu.addAction(act_open)
        self.recent_menu = QMenu("Recent Files", self)
        file_menu.addMenu(self.recent_menu)
        self._update_recent_menu()
        file_menu.addSeparator()
        act_save = QAction("&Save", self)
        act_save.setShortcut(QKeySequence.Save)
        act_save.triggered.connect(self.save_file)
        file_menu.addAction(act_save)
        act_save_as = QAction("Save &As...", self)
        act_save_as.setShortcut(QKeySequence("Ctrl+Shift+S"))
        act_save_as.triggered.connect(self.save_file_as)
        file_menu.addAction(act_save_as)
        file_menu.addSeparator()
        act_close = QAction("Close Tab", self)
        act_close.setShortcut(QKeySequence("Ctrl+W"))
        act_close.triggered.connect(lambda: self.close_tab(self.tabs.currentIndex()))
        file_menu.addAction(act_close)
        file_menu.addSeparator()
        act_quit = QAction("Quit", self)
        act_quit.setShortcut(QKeySequence.Quit)
        act_quit.triggered.connect(self.close)
        file_menu.addAction(act_quit)
        edit_menu = menubar.addMenu("&Edit")
        act_undo = QAction("Undo", self)
        act_undo.setShortcut(QKeySequence.Undo)
        act_undo.triggered.connect(lambda: self._call_editor("undo"))
        edit_menu.addAction(act_undo)
        act_redo = QAction("Redo", self)
        act_redo.setShortcut(QKeySequence.Redo)
        act_redo.triggered.connect(lambda: self._call_editor("redo"))
        edit_menu.addAction(act_redo)
        edit_menu.addSeparator()
        act_cut = QAction("Cut", self)
        act_cut.setShortcut(QKeySequence.Cut)
        act_cut.triggered.connect(lambda: self._call_editor("cut"))
        edit_menu.addAction(act_cut)
        act_copy = QAction("Copy", self)
        act_copy.setShortcut(QKeySequence.Copy)
        act_copy.triggered.connect(lambda: self._call_editor("copy"))
        edit_menu.addAction(act_copy)
        act_paste = QAction("Paste", self)
        act_paste.setShortcut(QKeySequence.Paste)
        act_paste.triggered.connect(lambda: self._call_editor("paste"))
        edit_menu.addAction(act_paste)
        edit_menu.addSeparator()
        act_find = QAction("Find && Replace...", self)
        act_find.setShortcut(QKeySequence.Find)
        act_find.triggered.connect(self.show_search)
        edit_menu.addAction(act_find)
        edit_menu.addSeparator()
        act_settings = QAction("⚙ Settings...", self)
        act_settings.setShortcut(QKeySequence("Ctrl+,"))
        act_settings.triggered.connect(self.show_settings)
        edit_menu.addAction(act_settings)
        run_menu = menubar.addMenu("&Run")
        act_run = QAction("Run Script", self)
        act_run.setShortcut(QKeySequence("F5"))
        act_run.triggered.connect(self.run_script)
        run_menu.addAction(act_run)
        act_stop = QAction("Stop", self)
        act_stop.triggered.connect(self.stop_script)
        run_menu.addAction(act_stop)
        run_menu.addSeparator()
        act_clear = QAction("Clear Output", self)
        act_clear.setShortcut(QKeySequence("Ctrl+L"))
        act_clear.triggered.connect(self.output.clear_output)
        run_menu.addAction(act_clear)
        view_menu = menubar.addMenu("&View")
        theme_menu = QMenu("Theme", self)
        self.theme_actions = {}
        for name in THEMES:
            act = QAction(name, self)
            act.setCheckable(True)
            act.setChecked(name == self.current_theme_name)
            act.triggered.connect(lambda checked, n=name: self.change_theme(n))
            theme_menu.addAction(act)
            self.theme_actions[name] = act
        view_menu.addMenu(theme_menu)
        view_menu.addSeparator()
        tool_window_menu = QMenu("Tool Windows", self)
        view_menu.addMenu(tool_window_menu)
        self.act_toggle_ref = QAction("Reference", self)
        self.act_toggle_ref.setCheckable(True)
        self.act_toggle_ref.setChecked(True)
        self.act_toggle_ref.triggered.connect(lambda c: self._toggle_dock(self.ref_dock, c))
        tool_window_menu.addAction(self.act_toggle_ref)
        self.act_toggle_blueprints = QAction("Blueprints", self)
        self.act_toggle_blueprints.setCheckable(True)
        self.act_toggle_blueprints.setChecked(True)
        self.act_toggle_blueprints.triggered.connect(lambda c: self._toggle_dock(self.blueprints_dock, c))
        tool_window_menu.addAction(self.act_toggle_blueprints)
        self.act_toggle_algorithm = QAction("Algorithm Viewer", self)
        self.act_toggle_algorithm.setCheckable(True)
        self.act_toggle_algorithm.setChecked(True)
        self.act_toggle_algorithm.triggered.connect(lambda c: self._toggle_dock(self.algorithm_dock, c))
        tool_window_menu.addAction(self.act_toggle_algorithm)
        self.act_toggle_visual = QAction("Visual Canvas", self)
        self.act_toggle_visual.setCheckable(True)
        self.act_toggle_visual.setChecked(True)
        self.act_toggle_visual.triggered.connect(lambda c: self._toggle_dock(self.visual_dock, c))
        tool_window_menu.addAction(self.act_toggle_visual)
        self.act_toggle_terminal = QAction("Terminal", self)
        self.act_toggle_terminal.setCheckable(True)
        self.act_toggle_terminal.setChecked(True)
        self.act_toggle_terminal.triggered.connect(lambda c: self._toggle_dock(self.terminal_dock, c))
        tool_window_menu.addAction(self.act_toggle_terminal)
        self.act_toggle_output = QAction("Output", self)
        self.act_toggle_output.setCheckable(True)
        self.act_toggle_output.setChecked(True)
        self.act_toggle_output.triggered.connect(lambda c: self.output.setVisible(c))
        tool_window_menu.addAction(self.act_toggle_output)
        self.ref_dock.visibilityChanged.connect(lambda v: self._sync_action(self.act_toggle_ref, v))
        self.blueprints_dock.visibilityChanged.connect(lambda v: self._sync_action(self.act_toggle_blueprints, v))
        self.algorithm_dock.visibilityChanged.connect(lambda v: self._sync_action(self.act_toggle_algorithm, v))
        self.visual_dock.visibilityChanged.connect(lambda v: self._sync_action(self.act_toggle_visual, v))
        self.terminal_dock.visibilityChanged.connect(lambda v: self._sync_action(self.act_toggle_terminal, v))
        help_menu = menubar.addMenu("&Help")
        act_about = QAction("About Vulpin IDE", self)
        act_about.triggered.connect(self.show_about)
        help_menu.addAction(act_about)
        act_vulpin = QAction("Vulpin GitHub", self)
        act_vulpin.triggered.connect(lambda: QDesktopServices.openUrl(QUrl("https://github.com/vulpin-lang/vulpin")))
        help_menu.addAction(act_vulpin)
        act_shortcuts = QAction("Keyboard Shortcuts", self)
        act_shortcuts.triggered.connect(self.show_shortcuts)
        help_menu.addAction(act_shortcuts)

    def _build_toolbar(self):
        toolbar = QToolBar("Main Toolbar")
        toolbar.setMovable(False)
        toolbar.setFloatable(False)
        toolbar.setIconSize(QSize(16, 16))
        toolbar.setToolButtonStyle(Qt.ToolButtonTextOnly)
        self.addToolBar(toolbar)
        act_new = QAction("New", self)
        act_new.triggered.connect(self.new_file)
        toolbar.addAction(act_new)
        act_open = QAction("Open", self)
        act_open.triggered.connect(self.open_file_dialog)
        toolbar.addAction(act_open)
        act_save = QAction("Save", self)
        act_save.triggered.connect(self.save_file)
        toolbar.addAction(act_save)
        toolbar.addSeparator()
        act_run = QAction("Run", self)
        act_run.triggered.connect(self.run_script)
        toolbar.addAction(act_run)
        act_stop = QAction("Stop", self)
        act_stop.triggered.connect(self.stop_script)
        toolbar.addAction(act_stop)
        toolbar.addSeparator()
        toolbar.addAction(self.act_toggle_blueprints)
        toolbar.addAction(self.act_toggle_algorithm)
        toolbar.addAction(self.act_toggle_visual)
        toolbar.addAction(self.act_toggle_terminal)
        toolbar.addSeparator()
        toolbar.addWidget(QLabel("  Theme "))
        self.theme_combo = QComboBox()
        self.theme_combo.addItems(list(THEMES.keys()))
        self.theme_combo.setCurrentText(self.current_theme_name)
        self.theme_combo.currentTextChanged.connect(self.change_theme)
        self.theme_combo.setMinimumWidth(220)
        toolbar.addWidget(self.theme_combo)
        self.toolbar = toolbar

    def _build_statusbar(self):
        self.statusbar = QStatusBar()
        self.setStatusBar(self.statusbar)
        self.lbl_position = QLabel("Ln 1, Col 1")
        self.lbl_position.setMinimumWidth(100)
        self.statusbar.addPermanentWidget(self.lbl_position)
        self.lbl_file = QLabel("untitled.vul")
        self.lbl_file.setMinimumWidth(200)
        self.statusbar.addPermanentWidget(self.lbl_file)
        self.lbl_encoding = QLabel("UTF-8")
        self.statusbar.addPermanentWidget(self.lbl_encoding)
        self.lbl_theme = QLabel(self.current_theme_name)
        self.statusbar.addPermanentWidget(self.lbl_theme)
        profile = self.settings.get("power_profile", "balanced").capitalize()
        self.lbl_power = QLabel(f"⚡ {profile}")
        self.statusbar.addPermanentWidget(self.lbl_power)

    def _apply_global_theme(self):
        t = self.theme
        self.setStyleSheet(f"""
            QMainWindow, QDialog {{ background-color: {t['bg']}; color: {t['fg']}; }}
            QMenuBar {{ background-color: {t['menu_bg']}; color: {t['menu_fg']}; border-bottom: 1px solid {t['gutter_border']}; padding: 2px; }}
            QMenuBar::item {{ padding: 4px 8px; background: transparent; border-radius: 3px; }}
            QMenuBar::item:selected {{ background-color: {t['sel']}; }}
            QMenuBar::item:pressed {{ background-color: {t.get('accent', '#89b4fa')}; color: #ffffff; }}
            QMenu {{ background-color: {t['menu_bg']}; color: {t['menu_fg']}; border: 1px solid {t['gutter_border']}; padding: 4px 0; }}
            QMenu::item {{ padding: 5px 24px; border-radius: 3px; margin: 1px 4px; }}
            QMenu::item:selected {{ background-color: {t['sel']}; }}
            QMenu::item:pressed {{ background-color: {t.get('accent', '#89b4fa')}; color: #ffffff; }}
            QMenu::separator {{ height: 1px; background: {t['gutter_border']}; margin: 4px 8px; }}
            QToolBar {{ background-color: {t['toolbar_bg']}; color: {t['toolbar_fg']}; border-bottom: 1px solid {t['gutter_border']}; spacing: 2px; padding: 3px; }}
            QToolBar QToolButton {{ color: {t['toolbar_fg']}; padding: 4px 8px; border-radius: 4px; background: transparent; }}
            QToolBar QToolButton:hover {{ background-color: {t['sel']}; }}
            QToolBar QToolButton:checked {{ background-color: {t['sel']}; border: 1px solid {t['gutter_border']}; }}
            QToolBar QToolButton:pressed {{ background-color: {t.get('accent', '#89b4fa')}; color: #ffffff; }}
            QTabWidget::pane {{ border: 1px solid {t['gutter_border']}; background: {t['bg']}; }}
            QTabBar {{ background: transparent; }}
            QTabBar::tab {{ background: {t['tab_bg']}; color: {t['line_fg']}; padding: 6px 14px; border: none; border-right: 1px solid {t['gutter_border']}; min-width: 80px; }}
            QTabBar::tab:selected {{ background: {t['tab_active']}; color: {t['fg']}; border-bottom: 2px solid {t.get('accent', '#89b4fa')}; }}
            QTabBar::tab:hover:!selected {{ background: {t['sel']}; color: {t['fg']}; }}
            QTabBar::close-button {{ border: none; background: transparent; border-radius: 3px; }}
            QTabBar::close-button:hover {{ background: {t['builtin']}; }}
            QDockWidget {{ background-color: {t['dock_bg']}; color: {t['dock_fg']}; }}
            QDockWidget::title {{ background: {t['toolbar_bg']}; color: {t['dock_fg']}; padding: 5px 8px; border-bottom: 1px solid {t['gutter_border']}; }}
            QDockWidget::close-button, QDockWidget::float-button {{ background: transparent; border: none; padding: 2px; border-radius: 3px; }}
            QDockWidget::close-button:hover, QDockWidget::float-button:hover {{ background: {t['sel']}; }}
            QStatusBar {{ background-color: {t['statusbar_bg']}; color: {t['statusbar_fg']}; border-top: 1px solid {t['gutter_border']}; font-size: 11px; }}
            QStatusBar::item {{ border: none; }}
            QComboBox {{ background-color: {t['toolbar_bg']}; color: {t['toolbar_fg']}; border: 1px solid {t['gutter_border']}; border-radius: 4px; padding: 3px 8px; min-height: 20px; }}
            QComboBox:hover {{ border-color: {t.get('accent', '#89b4fa')}; }}
            QComboBox::drop-down {{ border: none; width: 18px; }}
            QComboBox QAbstractItemView {{ background-color: {t['autocomplete_bg']}; color: {t['autocomplete_fg']}; selection-background-color: {t['autocomplete_sel']}; border: 1px solid {t['gutter_border']}; outline: none; }}
            QSplitter::handle {{ background-color: {t['gutter_border']}; height: 3px; width: 3px; }}
            QLineEdit {{ background-color: {t['toolbar_bg']}; color: {t['fg']}; border: 1px solid {t['gutter_border']}; border-radius: 4px; padding: 5px; selection-background-color: {t['sel']}; }}
            QLineEdit:focus {{ border-color: {t.get('accent', '#89b4fa')}; }}
            QPushButton {{ background-color: {t['toolbar_bg']}; color: {t['fg']}; border: 1px solid {t['gutter_border']}; border-radius: 4px; padding: 5px 14px; }}
            QPushButton:hover {{ background-color: {t['sel']}; border-color: {t.get('accent', '#89b4fa')}; }}
            QPushButton:pressed {{ background-color: {t.get('accent', '#89b4fa')}; color: #ffffff; }}
            QCheckBox, QLabel {{ color: {t['fg']}; background: transparent; }}
            QTextEdit {{ background-color: {t['output_bg']}; color: {t['output_fg']}; border: none; border-top: 1px solid {t['gutter_border']}; }}
            QTreeWidget {{ background-color: {t['dock_bg']}; color: {t['dock_fg']}; border: none; outline: none; alternate-background-color: {t['bg']}; }}
            QTreeWidget::item {{ padding: 3px; border: none; border-radius: 3px; }}
            QTreeWidget::item:hover {{ background-color: {t['sel']}; }}
            QTreeWidget::item:selected {{ background-color: {t['sel']}; }}
            QHeaderView::section {{ background-color: {t['toolbar_bg']}; color: {t['toolbar_fg']}; border: none; border-bottom: 1px solid {t['gutter_border']}; padding: 4px 6px; font-weight: bold; }}
            QScrollBar:vertical {{ background: transparent; width: 10px; margin: 0; }}
            QScrollBar::handle:vertical {{ background: {t['scrollbar']}; min-height: 30px; border-radius: 5px; }}
            QScrollBar::handle:vertical:hover {{ background: {t['scrollbar_hover']}; }}
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {{ height: 0; }}
            QScrollBar:horizontal {{ background: transparent; height: 10px; margin: 0; }}
            QScrollBar::handle:horizontal {{ background: {t['scrollbar']}; min-width: 30px; border-radius: 5px; }}
            QScrollBar::handle:horizontal:hover {{ background: {t['scrollbar_hover']}; }}
            QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal {{ width: 0; }}
            QCompleter QAbstractItemView {{ background-color: {t['autocomplete_bg']}; color: {t['autocomplete_fg']}; selection-background-color: {t['autocomplete_sel']}; border: 1px solid {t['gutter_border']}; outline: none; }}
            QToolTip {{ background-color: {t['autocomplete_bg']}; color: {t['autocomplete_fg']}; border: 1px solid {t['gutter_border']}; padding: 3px; }}
            QGraphicsView {{ border: none; background: transparent; }}
            QSpinBox {{ background-color: {t['toolbar_bg']}; color: {t['fg']}; border: 1px solid {t['gutter_border']}; border-radius: 4px; padding: 3px 8px; }}
            QRadioButton {{ color: {t['fg']}; }}
            QGroupBox {{ color: {t['fg']}; border: 1px solid {t['gutter_border']}; border-radius: 4px; margin-top: 10px; padding-top: 10px; }}
            QGroupBox::title {{ subcontrol-origin: margin; left: 10px; padding: 0 5px; }}
            QDialogButtonBox QPushButton {{ min-width: 80px; }}
        """)

    def change_theme(self, name):
        if name not in THEMES:
            return
        self.current_theme_name = name
        self.theme = THEMES[name]
        self.settings["theme"] = name
        for i in range(self.tabs.count()):
            editor = self.tabs.widget(i)
            if isinstance(editor, CodeEditor):
                editor.set_theme(self.theme)
        self.output.set_theme(self.theme)
        self.ref_dock.set_theme(self.theme)
        self.blueprints_dock.set_theme(self.theme)
        self.algorithm_dock.set_theme(self.theme)
        self.visual_dock.set_theme(self.theme)
        self.terminal_dock.set_theme(self.theme)
        self.theme_combo.blockSignals(True)
        self.theme_combo.setCurrentText(name)
        self.theme_combo.blockSignals(False)
        for tname, act in self.theme_actions.items():
            act.setChecked(tname == name)
        self.lbl_theme.setText(name)
        self._apply_global_theme()
        self._refresh_algorithm()
        self._save_settings()

    def _current_editor(self):
        w = self.tabs.currentWidget()
        return w if isinstance(w, CodeEditor) else None

    def _call_editor(self, method_name):
        editor = self._current_editor()
        if editor and hasattr(editor, method_name):
            getattr(editor, method_name)()

    def _create_editor(self):
        editor = CodeEditor(self.theme, self.settings)
        editor.cursorMoved.connect(self._update_status_position)
        editor.modificationChanged.connect(self._on_modification_changed)
        editor.textChanged.connect(self._schedule_algorithm_refresh)
        return editor

    def new_file(self):
        editor = self._create_editor()
        idx = self.tabs.addTab(editor, "untitled.vul")
        self.tabs.setCurrentIndex(idx)
        editor.setFocus()

    def open_file_dialog(self):
        path, _ = QFileDialog.getOpenFileName(self, "Open Vulpin File", "", "Vulpin Files (*.vul);;All Files (*)")
        if path:
            self.open_file(path)

    def open_file(self, path):
        for i in range(self.tabs.count()):
            editor = self.tabs.widget(i)
            if isinstance(editor, CodeEditor) and editor.file_path == path:
                self.tabs.setCurrentIndex(i)
                return
        try:
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Cannot open file:\n{e}")
            return
        editor = self._create_editor()
        editor.setPlainText(content)
        editor.file_path = path
        editor.is_modified = False
        editor.document().setModified(False)
        name = os.path.basename(path)
        idx = self.tabs.addTab(editor, name)
        self.tabs.setCurrentIndex(idx)
        editor.setFocus()
        if path in self.recent_files:
            self.recent_files.remove(path)
        self.recent_files.insert(0, path)
        limit = self.settings.get("recent_files_limit", 10)
        self.recent_files = self.recent_files[:limit]
        self._update_recent_menu()
        self._save_settings()
        self.lbl_file.setText(name)
        self.output.write_ok(f"Opened: {path}")
        self._schedule_algorithm_refresh()

    def save_file(self):
        editor = self._current_editor()
        if not editor:
            return
        if editor.file_path:
            self._write_file(editor, editor.file_path)
        else:
            self.save_file_as()

    def save_file_as(self):
        editor = self._current_editor()
        if not editor:
            return
        path, _ = QFileDialog.getSaveFileName(self, "Save Vulpin File", "", "Vulpin Files (*.vul);;All Files (*)")
        if path:
            if not path.endswith(".vul"):
                path += ".vul"
            self._write_file(editor, path)

    def _write_file(self, editor, path):
        try:
            with open(path, "w", encoding="utf-8") as f:
                f.write(editor.toPlainText())
            editor.file_path = path
            editor.is_modified = False
            editor.document().setModified(False)
            name = os.path.basename(path)
            self.tabs.setTabText(self.tabs.currentIndex(), name)
            self.lbl_file.setText(name)
            self.output.write_ok(f"Saved: {path}")
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Cannot save file:\n{e}")

    def close_tab(self, index):
        if index < 0:
            return
        editor = self.tabs.widget(index)
        if isinstance(editor, CodeEditor) and editor.is_modified:
            if self.settings.get("confirm_on_close", True):
                reply = QMessageBox.question(self, "Unsaved Changes", f"Save changes to {self.tabs.tabText(index)}?", QMessageBox.Save | QMessageBox.Discard | QMessageBox.Cancel)
                if reply == QMessageBox.Save:
                    self.tabs.setCurrentIndex(index)
                    self.save_file()
                elif reply == QMessageBox.Cancel:
                    return
        self.tabs.removeTab(index)
        if self.tabs.count() == 0:
            self.new_file()

    def _on_tab_changed(self, index):
        editor = self._current_editor()
        if editor:
            self.lbl_file.setText(os.path.basename(editor.file_path) if editor.file_path else "untitled.vul")
        self._schedule_algorithm_refresh()

    def _on_modification_changed(self, modified):
        editor = self._current_editor()
        if editor:
            idx = self.tabs.currentIndex()
            name = self.tabs.tabText(idx)
            if modified and not name.endswith(" *"):
                self.tabs.setTabText(idx, name + " *")
            elif not modified and name.endswith(" *"):
                self.tabs.setTabText(idx, name[:-2])

    def _update_status_position(self, line, col):
        self.lbl_position.setText(f"Ln {line}, Col {col}")

    def show_search(self):
        editor = self._current_editor()
        if editor:
            SearchReplaceDialog(editor, self).show()

    def show_settings(self):
        dlg = SettingsDialog(self.settings, THEMES, self)
        if dlg.exec_() == QDialog.Accepted:
            new_settings = dlg.get_settings()
            self._apply_settings(new_settings)

    def _apply_settings(self, new_settings):
        old_theme = self.settings.get("theme")
        old_profile = self.settings.get("power_profile")
        
        self.settings.update(new_settings)
        
        # Apply theme change
        if new_settings.get("theme") != old_theme:
            self.change_theme(new_settings["theme"])
        
        # Apply power profile change
        if new_settings.get("power_profile") != old_profile:
            self._algo_timer.setInterval(new_settings.get("algo_refresh_ms", 400))
            profile = new_settings.get("power_profile", "balanced").capitalize()
            self.lbl_power.setText(f"⚡ {profile}")
        
        # Apply editor settings to all editors
        for i in range(self.tabs.count()):
            editor = self.tabs.widget(i)
            if isinstance(editor, CodeEditor):
                editor.apply_settings(self.settings)
        
        # Apply output height
        self.output.setMaximumHeight(self.settings.get("output_height", 220))
        
        self._save_settings()

    def run_script(self):
        editor = self._current_editor()
        if not editor:
            return
        if hasattr(self, "_process") and self._process and self._process.state() != QProcess.NotRunning:
            self.output.write_err("A script is already running. Stop it first.")
            return
        is_temp = False
        if editor.file_path:
            self.save_file()
            script_path = editor.file_path
        else:
            tmp = tempfile.NamedTemporaryFile(suffix=".vul", delete=False, mode="w", encoding="utf-8")
            tmp.write(editor.toPlainText())
            tmp.close()
            script_path = tmp.name
            is_temp = True
        self.output.clear_output()
        self.output.write(f"Running: {os.path.basename(script_path)}")
        self.output.write("─" * 50)
        script_dir = os.path.dirname(os.path.abspath(script_path))
        
        vulpin_path = self.settings.get("vulpin_path", "")
        if vulpin_path and os.path.exists(vulpin_path):
            cmd = [vulpin_path, script_path]
        else:
            vulpin_exe = shutil.which("vulpin")
            if vulpin_exe:
                cmd = [vulpin_exe, script_path]
            else:
                candidates = [
                    ["vulpin", script_path],
                    [sys.executable, "-m", "vulpin", script_path],
                    [sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "vulpin.py"), script_path],
                    [sys.executable, "vulpin.py", script_path],
                ]
                cmd = None
                for c in candidates:
                    if shutil.which(c[0]) or os.path.exists(c[0]):
                        cmd = c
                        break
                if not cmd:
                    cmd = candidates[0]
        
        self._process = QProcess(self)
        self._process.setWorkingDirectory(script_dir)
        self._process.setProcessChannelMode(QProcess.SeparateChannels)
        self._is_temp_script = is_temp
        self._temp_script_path = script_path
        self._process.readyReadStandardOutput.connect(self._read_stdout)
        self._process.readyReadStandardError.connect(self._read_stderr)
        self._process.finished.connect(self._process_finished)
        self._process.errorOccurred.connect(self._process_error)
        self.output.write("$ " + " ".join(cmd))
        self._process.start(cmd[0], cmd[1:])
        if self._process.state() == QProcess.NotRunning:
            self.output.write_err("Failed to start process.")
            self.output.set_input_enabled(False)
            if is_temp:
                try:
                    os.unlink(script_path)
                except Exception:
                    pass
        else:
            self.output.set_input_enabled(True)
            self.output.input.setFocus()

    def _read_stdout(self):
        if not hasattr(self, "_process"):
            return
        data = self._process.readAllStandardOutput().data()
        try:
            text = data.decode("utf-8", errors="replace")
        except Exception:
            text = str(data)
        if text:
            self.output.write(text.rstrip("\n"))

    def _read_stderr(self):
        if not hasattr(self, "_process"):
            return
        data = self._process.readAllStandardError().data()
        try:
            text = data.decode("utf-8", errors="replace")
        except Exception:
            text = str(data)
        if text:
            self.output.write_err(text.rstrip("\n"))

    def _process_finished(self, exit_code, exit_status):
        self.output.write("─" * 50)
        if exit_code == 0:
            self.output.write_ok(f"Process finished with exit code 0")
        else:
            self.output.write_err(f"Process finished with exit code {exit_code}")
        self.output.set_input_enabled(False)
        if hasattr(self, "_is_temp_script") and self._is_temp_script and hasattr(self, "_temp_script_path"):
            try:
                os.unlink(self._temp_script_path)
            except Exception:
                pass

    def _process_error(self, error):
        errors = {
            QProcess.FailedToStart: "Failed to start. Is 'vulpin' installed and in PATH?",
            QProcess.Crashed: "Process crashed.",
            QProcess.Timedout: "Process timed out.",
            QProcess.WriteError: "Write error.",
            QProcess.ReadError: "Read error.",
            QProcess.UnknownError: "Unknown error.",
        }
        msg = errors.get(error, "Unknown error.")
        self.output.write_err(msg)
        self.output.set_input_enabled(False)

    def _send_input_to_process(self, text):
        if not hasattr(self, "_process") or not self._process or self._process.state() == QProcess.NotRunning:
            self.output.write_err("No process is running.")
            return
        try:
            data = (text + "\n").encode("utf-8")
            self._process.write(data)
        except Exception as e:
            self.output.write_err(f"Failed to send input: {e}")

    def stop_script(self):
        if hasattr(self, "_process") and self._process and self._process.state() != QProcess.NotRunning:
            self._process.kill()
            self._process.waitForFinished(2000)
            self.output.write_err("Process stopped.")
            self.output.set_input_enabled(False)

    def _insert_from_dock(self, text):
        editor = self._current_editor()
        if editor:
            editor.textCursor().insertText(text)
            editor.setFocus()

    def _sync_blueprint_code(self, code):
        editor = self._current_editor()
        if editor:
            editor._programmatic_change = True
            try:
                editor.setPlainText(code)
            finally:
                editor._programmatic_change = False

    def _toggle_dock(self, dock, visible):
        dock.setVisible(visible)
        if visible:
            dock.raise_()

    def _sync_action(self, action, checked):
        action.blockSignals(True)
        action.setChecked(checked)
        action.blockSignals(False)

    def _schedule_algorithm_refresh(self):
        self._algo_timer.start()

    def _refresh_algorithm(self):
        editor = self._current_editor()
        if editor:
            self.algorithm_dock.refresh(editor.toPlainText())
        else:
            self.algorithm_dock.tree.clear()

    def _update_recent_menu(self):
        self.recent_menu.clear()
        if not self.recent_files:
            act = QAction("(No recent files)", self)
            act.setEnabled(False)
            self.recent_menu.addAction(act)
            return
        for path in self.recent_files:
            name = os.path.basename(path)
            act = QAction(f"{name}  —  {path}", self)
            act.triggered.connect(lambda checked, p=path: self.open_file(p))
            self.recent_menu.addAction(act)

    def show_about(self):
        QMessageBox.about(
            self,
            "About Vulpin IDE",
            "Vulpin IDE v1.8.0\n\n"
            "A beautiful IDE for the Vulpin programming language.\n\n"
            "Features:\n"
            "- Unreal-style Blueprint node editor\n"
            "- Visual Basic-style form designer (theme-adaptive)\n"
            "- Interactive Console with input support\n"
            "- Auto sync graph to code\n"
            "- Editable nodes\n"
            "- Terminal\n"
            "- Algorithm viewer\n"
            "- Settings with Power Profiles\n"
            "- Syntax highlighting\n"
            "- Themes\n\n"
            "https://github.com/vulpin-lang/vulpin"
        )

    def show_shortcuts(self):
        shortcuts = (
            "Ctrl+N - New file\n"
            "Ctrl+O - Open file\n"
            "Ctrl+S - Save\n"
            "Ctrl+Shift+S - Save As\n"
            "Ctrl+W - Close tab\n"
            "Ctrl+Z - Undo\n"
            "Ctrl+Y - Redo\n"
            "Ctrl+X/C/V - Cut / Copy / Paste\n"
            "Ctrl+F - Search & Replace\n"
            "Ctrl+, - Settings\n"
            "F5 - Run script\n"
            "Ctrl+L - Clear output\n"
            "Tab - Indent\n"
            "Shift+Tab - Unindent"
        )
        QMessageBox.information(self, "Keyboard Shortcuts", shortcuts)

    def dragEnterEvent(self, event):
        if event.mimeData().hasUrls():
            for url in event.mimeData().urls():
                if url.toLocalFile().endswith(".vul"):
                    event.acceptProposedAction()
                    return

    def dropEvent(self, event):
        for url in event.mimeData().urls():
            path = url.toLocalFile()
            if path.endswith(".vul"):
                self.open_file(path)

    def closeEvent(self, event):
        if hasattr(self, "_process") and self._process and self._process.state() != QProcess.NotRunning:
            reply = QMessageBox.question(
                self, "Running Script",
                "A script is still running. Stop it and quit?",
                QMessageBox.Yes | QMessageBox.No
            )
            if reply == QMessageBox.Yes:
                self._process.kill()
                self._process.waitForFinished(2000)
            else:
                event.ignore()
                return
        if self.settings.get("confirm_on_close", True):
            for i in range(self.tabs.count()):
                editor = self.tabs.widget(i)
                if isinstance(editor, CodeEditor) and editor.is_modified:
                    reply = QMessageBox.question(self, "Unsaved Changes", "You have unsaved changes. Quit anyway?", QMessageBox.Save | QMessageBox.Discard | QMessageBox.Cancel)
                    if reply == QMessageBox.Save:
                        self.tabs.setCurrentIndex(i)
                        self.save_file()
                    elif reply == QMessageBox.Cancel:
                        event.ignore()
                        return
        self._save_settings()
        event.accept()


def main():
    QApplication.setAttribute(Qt.AA_EnableHighDpiScaling, True)
    QApplication.setAttribute(Qt.AA_UseHighDpiPixmaps, True)
    app = QApplication(sys.argv)
    style = QStyleFactory.create("Fusion")
    if style:
        app.setStyle(style)
    app.setApplicationName("Vulpin IDE")
    app.setOrganizationName("VulpinLang")
    ui_font = QFont("Segoe UI", 10)
    ui_font.setStyleHint(QFont.SansSerif)
    app.setFont(ui_font)
    window = VulpinIDE()
    window.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()