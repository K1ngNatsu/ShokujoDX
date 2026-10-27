// The Quick Look thumbnail extension of the macOS app (P11 Milestone 3,
// ADR 0046): Finder shows the picture of project and machine files.
//
// The extension runs in a sandbox. It reads the file (Quick Look gives it
// the right to), gives the bytes to the helper `emb-thumbnail-helper` next
// to it on standard input, and draws the PNG that the helper writes. The
// helper is signed to inherit the sandbox and opens no file.

import AppKit
import QuickLookThumbnailing

class ThumbnailProvider: QLThumbnailProvider {
    override func provideThumbnail(
        for request: QLFileThumbnailRequest,
        _ handler: @escaping (QLThumbnailReply?, Error?) -> Void
    ) {
        guard let helper = Bundle.main.url(forAuxiliaryExecutable: "emb-thumbnail-helper"),
              let data = try? Data(contentsOf: request.fileURL)
        else {
            handler(nil, nil)
            return
        }
        let pixels = Int(max(request.maximumSize.width, request.maximumSize.height) * request.scale)
        let process = Process()
        process.executableURL = helper
        process.arguments = [String(max(pixels, 1)), request.fileURL.pathExtension.lowercased()]
        let input = Pipe()
        let output = Pipe()
        process.standardInput = input
        process.standardOutput = output
        do {
            try process.run()
        } catch {
            handler(nil, error)
            return
        }
        // Write in the background, so that a full pipe does not stop the read.
        DispatchQueue.global().async {
            input.fileHandleForWriting.write(data)
            try? input.fileHandleForWriting.close()
        }
        let png = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0, let image = NSImage(data: png), image.size.width > 0 else {
            handler(nil, nil)
            return
        }
        // Fit the picture in the size that Quick Look asks for.
        let box = request.maximumSize
        let k = min(box.width / image.size.width, box.height / image.size.height)
        let size = CGSize(width: image.size.width * k, height: image.size.height * k)
        handler(
            QLThumbnailReply(contextSize: size, currentContextDrawing: { () -> Bool in
                image.draw(in: CGRect(origin: .zero, size: size))
                return true
            }),
            nil
        )
    }
}
