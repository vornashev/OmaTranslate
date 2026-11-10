import QtQuick
import QtQuick.Shapes

// 16-unit line icons shared with the OmaTranslate design file (page «Компоненты»).
Item {
    id: root
    property string name: ""
    property color color: "#e3e7e8"
    property real size: 16
    implicitWidth: size
    implicitHeight: size

    readonly property var paths: ({
        "copy": "M5.5 5.5h7v7h-7z M3.5 10.5v-7h7",
        "paste": "M5.5 2h5v2.5h-5z M4.5 3.25H3v10.25h10V3.25h-1.5",
        "region": "M2 5.5V2h3.5 M10.5 2H14v3.5 M14 10.5V14h-3.5 M5.5 14H2v-3.5",
        "swap": "M2.5 5h10 M10 2.5L12.5 5L10 7.5 M13.5 11h-10 M6 8.5L3.5 11L6 13.5",
        "close": "M4 4l8 8 M12 4l-8 8",
        "sliders": "M2.5 4.5h5 M10.5 4.5h3 M2.5 11.5h2 M7.5 11.5h6 M9 3v3 M6 10v3",
        "history": "M8 1.75a6.25 6.25 0 1 0 0.01 0 M8 4.5V8l2.5 1.5",
        "retry": "M13 8a5 5 0 1 1-1.46-3.54 M13 2.25v2.5h-2.5",
        "chev": "M4.5 6.5L8 10l3.5-3.5",
        "check": "M3 8.5l3 3l7-7",
        "search": "M7 2.5a4.5 4.5 0 1 0 0.01 0 M10.4 10.4L13.5 13.5",
        "back": "M13 8H3.5 M7.5 4L3.5 8l4 4",
        "alert": "M8 2L14.5 13.5H1.5Z M8 6.5v3 M8 11.5v0.25",
        "download": "M8 2.5v8 M4.5 7L8 10.5L11.5 7 M3 13.5h10",
        "image": "M2.5 3h11v10h-11z M2.5 10.5l3.5-3.5l3 3l2-2l2.5 2.5",
        "pencil": "M10.5 2.75l2.75 2.75L6 12.75H3.25V10z",
        "globe": "M8 2a6 6 0 1 0 0.01 0 M2 8h12 M8 2c2.2 2 2.2 10 0 12 M8 2c-2.2 2-2.2 10 0 12",
        "cpu": "M4.5 4.5h7v7h-7z M6.5 2v2.5 M9.5 2v2.5 M6.5 11.5V14 M9.5 11.5V14 M2 6.5h2.5 M2 9.5h2.5 M11.5 6.5H14 M11.5 9.5H14",
        "keyboard": "M1.5 4h13v8h-13z M4 6.75h0.5 M6.5 6.75h0.5 M9 6.75h0.5 M11.5 6.75h0.5 M5 9.5h6",
        "trash": "M3 4.5h10 M6 4.5V2.75h4V4.5 M4.5 4.5l0.75 9h5.5l0.75-9",
        "shield": "M8 1.75l5 2v4c0 3-2.2 5.2-5 6.5c-2.8-1.3-5-3.5-5-6.5v-4z",
        "plus": "M8 3v10 M3 8h10",
        "minus": "M3 8h10",
        "fit": "M2.5 6V2.5H6 M10 2.5h3.5V6 M13.5 10v3.5H10 M6 13.5H2.5V10",
        "arrow": "M3 8h9.5 M9 4.5L12.5 8L9 11.5",
        "info": "M8 2a6 6 0 1 0 0.01 0 M8 7.25v4 M8 4.75v0.25",
        "server": "M2.5 2.5h11v4.5h-11z M2.5 9h11v4.5h-11z M5 4.75h0.25 M5 11.25h0.25",
        "cursor": "M3.5 2.5h3 M5 2.5v11 M3.5 13.5h3 M9 5.5h4.5 M9 8h4.5 M9 10.5h3",
        "translate": "M1.75 3.5h7 M5.25 2v1.5 M3.25 3.5c0.6 2.6 2.2 4.4 4.5 5.5 M7.25 3.5c-0.6 2.6-2.2 4.4-4.75 5.6 M8.25 14l3-7.5l3 7.5 M9.2 11.75h4.1"
    })

    Shape {
        width: 16
        height: 16
        scale: root.size / 16
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: root.color
            strokeWidth: 1.5
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
