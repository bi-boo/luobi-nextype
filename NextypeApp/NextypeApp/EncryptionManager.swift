//
//  EncryptionManager.swift
//  NextypeApp
//
//  加密管理器 - 兼容CryptoJS的AES加密格式
//

import Foundation
import CommonCrypto

class EncryptionManager {
    static let shared = EncryptionManager()
    
    private init() {}
    
    /// 使用密钥加密文本（兼容CryptoJS格式）
    /// - Parameters:
    ///   - text: 要加密的明文
    ///   - keyString: 加密密钥（字符串）
    /// - Returns: Base64编码的加密结果
    func encrypt(_ text: String, using keyString: String) throws -> String {
        guard let textData = text.data(using: .utf8) else {
            throw EncryptionError.invalidInput
        }

        // 生成随机salt（8字节）
        var salt = Data(count: 8)
        let result = salt.withUnsafeMutableBytes { saltBytes in
            SecRandomCopyBytes(kSecRandomDefault, 8, saltBytes.baseAddress!)
        }
        guard result == errSecSuccess else {
            throw EncryptionError.encryptionFailed
        }

        // 使用密码和salt派生key和IV（EVP_BytesToKey算法）
        let (key, iv) = deriveKeyAndIV(password: keyString, salt: salt)

        // 加密
        let encryptedData = try aesEncrypt(data: textData, key: key, iv: iv)

        // 构建CryptoJS格式：Salted__ + salt + 密文
        let salted = "Salted__".data(using: .utf8)!
        let combined = salted + salt + encryptedData

        // 返回Base64编码
        return combined.base64EncodedString()
    }
    
    /// 使用密钥解密文本（兼容CryptoJS格式）
    /// - Parameters:
    ///   - encryptedText: Base64编码的加密文本
    ///   - keyString: 解密密钥（字符串）
    /// - Returns: 解密后的明文
    func decrypt(_ encryptedText: String, using keyString: String) throws -> String {
        guard let combined = Data(base64Encoded: encryptedText) else {
            throw EncryptionError.invalidInput
        }
        
        // 检查CryptoJS格式：Salted__ + salt(8字节) + 密文
        let saltedPrefix = "Salted__".data(using: .utf8)!
        guard combined.count > saltedPrefix.count + 8 else {
            throw EncryptionError.invalidInput
        }
        
        // 验证"Salted__"前缀
        let prefix = combined.prefix(saltedPrefix.count)
        guard prefix == saltedPrefix else {
            throw EncryptionError.invalidInput
        }
        
        // 提取salt（8字节）
        let salt = combined.subdata(in: saltedPrefix.count..<(saltedPrefix.count + 8))
        
        // 提取密文
        let encryptedData = combined.suffix(from: saltedPrefix.count + 8)
        
        // 使用密码和salt派生key和IV
        let (key, iv) = deriveKeyAndIV(password: keyString, salt: salt)
        
        // 解密
        let decryptedData = try aesDecrypt(data: encryptedData, key: key, iv: iv)
        
        // 转为字符串
        guard let decryptedText = String(data: decryptedData, encoding: .utf8) else {
            throw EncryptionError.invalidOutput
        }
        
        return decryptedText
    }
    
    // MARK: - Private Methods
    
    /// 从密钥字符串和salt派生Key和IV（使用EVP_BytesToKey算法，完全兼容CryptoJS）
    /// 这是OpenSSL的标准算法
    private func deriveKeyAndIV(password: String, salt: Data) -> (key: Data, iv: Data) {
        let passwordData = Data(password.utf8)
        let keySize = 32  // 256位密钥
        let ivSize = 16   // 128位IV
        let totalSize = keySize + ivSize
        
        var derivedData = Data()
        var block = Data()
        
        // EVP_BytesToKey算法
        while derivedData.count < totalSize {
            // block = MD5(block + password + salt)
            let hashInput = block + passwordData + salt
            block = md5(hashInput)
            derivedData.append(block)
        }
        
        let key = derivedData.prefix(keySize)
        let iv = derivedData.dropFirst(keySize).prefix(ivSize)
        
        return (Data(key), Data(iv))
    }
    
    /// MD5哈希（用于密钥派生，兼容CryptoJS的EVP_BytesToKey算法）
    /// 注意：MD5在此处用于与CryptoJS兼容的密钥派生，不用于安全哈希
    @available(iOS, deprecated: 13.0, message: "MD5 is used for CryptoJS compatibility, not for security")
    private func md5(_ data: Data) -> Data {
        var digest = [UInt8](repeating: 0, count: Int(CC_MD5_DIGEST_LENGTH))
        data.withUnsafeBytes { bytes in
            _ = CC_MD5(bytes.baseAddress, CC_LONG(data.count), &digest)
        }
        return Data(digest)
    }
    
    /// AES加密（CBC模式，PKCS7填充）
    private func aesEncrypt(data: Data, key: Data, iv: Data) throws -> Data {
        let bufferSize = data.count + kCCBlockSizeAES128
        var buffer = Data(count: bufferSize)
        var numBytesEncrypted: size_t = 0
        
        let cryptStatus = data.withUnsafeBytes { dataBytes in
            key.withUnsafeBytes { keyBytes in
                iv.withUnsafeBytes { ivBytes in
                    buffer.withUnsafeMutableBytes { bufferBytes in
                        CCCrypt(
                            CCOperation(kCCEncrypt),
                            CCAlgorithm(kCCAlgorithmAES),
                            CCOptions(kCCOptionPKCS7Padding),
                            keyBytes.baseAddress, key.count,
                            ivBytes.baseAddress,
                            dataBytes.baseAddress, data.count,
                            bufferBytes.baseAddress, bufferSize,
                            &numBytesEncrypted
                        )
                    }
                }
            }
        }
        
        guard cryptStatus == kCCSuccess else {
            throw EncryptionError.encryptionFailed
        }
        
        return buffer.prefix(numBytesEncrypted)
    }
    
    /// AES解密（CBC模式，PKCS7填充）
    private func aesDecrypt(data: Data, key: Data, iv: Data) throws -> Data {
        let bufferSize = data.count + kCCBlockSizeAES128
        var buffer = Data(count: bufferSize)
        var numBytesDecrypted: size_t = 0
        
        let cryptStatus = data.withUnsafeBytes { dataBytes in
            key.withUnsafeBytes { keyBytes in
                iv.withUnsafeBytes { ivBytes in
                    buffer.withUnsafeMutableBytes { bufferBytes in
                        CCCrypt(
                            CCOperation(kCCDecrypt),
                            CCAlgorithm(kCCAlgorithmAES),
                            CCOptions(kCCOptionPKCS7Padding),
                            keyBytes.baseAddress, key.count,
                            ivBytes.baseAddress,
                            dataBytes.baseAddress, data.count,
                            bufferBytes.baseAddress, bufferSize,
                            &numBytesDecrypted
                        )
                    }
                }
            }
        }
        
        guard cryptStatus == kCCSuccess else {
            throw EncryptionError.decryptionFailed
        }
        
        return buffer.prefix(numBytesDecrypted)
    }
}

// MARK: - Error Types

enum EncryptionError: LocalizedError {
    case invalidInput
    case invalidOutput
    case encryptionFailed
    case decryptionFailed
    
    var errorDescription: String? {
        switch self {
        case .invalidInput:
            return "无效的输入数据"
        case .invalidOutput:
            return "无效的输出数据"
        case .encryptionFailed:
            return "加密失败"
        case .decryptionFailed:
            return "解密失败"
        }
    }
}
