import Photos
import UIKit

enum LcdMetrics {
    static func side(appearance: Int, scalePercent: Int) -> CGFloat {
        let scale = CGFloat(max(50, min(150, scalePercent))) / 100
        if appearance == 2 { return max(64, 256 * scale) }
        if appearance == 1 { return 280 }
        return 220
    }
}

/// Saves a PNG into the iMeets album. Creates the album when it is missing.
enum PhotoStore {
    static let albumTitle = "iMeets"

    static func savePNG(_ file: URL, completion: @escaping (Result<Void, PhotoError>) -> Void) {
        // Finding the album reads the library, so add-only authorization is not enough.
        PHPhotoLibrary.requestAuthorization(for: .readWrite) { status in
            guard status == .authorized || status == .limited else {
                completion(.failure(.permission))
                return
            }
            let existing = fetchAlbum()
            PHPhotoLibrary.shared().performChanges({
                let asset = PHAssetCreationRequest.forAsset()
                asset.addResource(with: .photo, fileURL: file, options: nil)
                guard let placeholder = asset.placeholderForCreatedAsset else { return }
                if let existing, let change = PHAssetCollectionChangeRequest(for: existing) {
                    change.addAssets([placeholder] as NSArray)
                } else {
                    let created = PHAssetCollectionChangeRequest.creationRequestForAssetCollection(withTitle: albumTitle)
                    created.addAssets([placeholder] as NSArray)
                }
            }, completionHandler: { ok, _ in
                completion(ok ? .success(()) : .failure(.unknown))
            })
        }
    }

    private static func fetchAlbum() -> PHAssetCollection? {
        let options = PHFetchOptions()
        options.predicate = NSPredicate(format: "title = %@", albumTitle)
        return PHAssetCollection.fetchAssetCollections(with: .album, subtype: .any, options: options).firstObject
    }
}

enum PhotoError: LocalizedError {
    case permission
    case unknown

    var errorDescription: String? {
        switch self {
        case .permission: return "permission"
        case .unknown: return "unknown"
        }
    }
}
