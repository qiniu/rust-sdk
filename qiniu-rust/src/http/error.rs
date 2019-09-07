use error_chain::error_chain;
use qiniu_http::StatusCode;

error_chain! {
    errors {
        BadRequestError(code: StatusCode, message: Box<str>) { // 400
            description("Bad request Error"),
            display("Bad request Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        UnauthorizedError(code: StatusCode, message: Box<str>) { // 401
            description("Unauthorized Error"),
            display("Unauthorized Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        ForbiddenError(code: StatusCode, message: Box<str>) { // 403
            description("Forbidden Error"),
            display("Forbidden Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        URLNotFoundError(code: StatusCode, message: Box<str>) { // 404
            description("URL Not Found Error"),
            display("URL Not Found Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        MethodNotAllowedError(code: StatusCode, message: Box<str>) { // 405
            description("Method not allowed Error"),
            display("Method not allowed Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        NotAcceptableError(code: StatusCode, message: Box<str>) { // 406
            description("Not acceptable Error"),
            display("Not acceptable Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        ConflictError(code: StatusCode, message: Box<str>) { // 409
            description("Conflict Error"),
            display("Conflict Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        UserDisabledError(code: StatusCode, message: Box<str>) { // 419
            description("User is disabled"),
            display("User is disabled: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        NotImplementedError(code: StatusCode, message: Box<str>) { // 501
            description("Not implemented Error"),
            display("Not implemented Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        BadGatewayError(code: StatusCode, message: Box<str>) { // 502
            description("Bad gateway Error"),
            display("Bad gateway Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        ServiceUnavailableError(code: StatusCode, message: Box<str>) { // 503
            description("Service unavailable Error"),
            display("Service unavailable Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        GatewayTimeoutError(code: StatusCode, message: Box<str>) { // 503
            description("Gateway timeout Error"),
            display("Gateway timeout Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        BusyError(code: StatusCode, message: Box<str>) { // 571
            description("Try later"),
            display("Try later: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        OutOfLimitError(code: StatusCode, message: Box<str>) { // 573
            description("Out of limit"),
            display("Out of limit: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        CallbackError(code: StatusCode, message: Box<str>) { // 579
            description("Everything is OK but callback was failed"),
            display("Everything is OK but callback was failed: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        InternalServerError(code: StatusCode, message: Box<str>) { // 599
            description("Internal Server Error"),
            display("Internal Server Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        FileModifiedError(code: StatusCode, message: Box<str>) { // 608
            description("File Modified Error"),
            display("File Modified Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        ResourceNotFoundError(code: StatusCode, message: Box<str>) { // 612
            description("Resource not found Error"),
            display("Resource not found Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        ResourceExistsError(code: StatusCode, message: Box<str>) { // 614
            description("Resource exists Error"),
            display("Resource exists Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        RoomIsInactiveError(code: StatusCode, message: Box<str>) { // 615
            description("Room is inactive Error"),
            display("Room is inactive Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        HubNotMatchError(code: StatusCode, message: Box<str>) { // 616
            description("Hub not match Error"),
            display("Hub not match Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        NoDataError(code: StatusCode, message: Box<str>) { // 619
            description("No data Error"),
            display("No data Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        TooManyBucketsError(code: StatusCode, message: Box<str>) { // 630
            description("Too many buckets Error"),
            display("Too many buckets Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        BucketNotFoundError(code: StatusCode, message: Box<str>) { // 631
            description("Bucket is not found Error"),
            display("Bucket is not found Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        InvalidMarkerError(code: StatusCode, message: Box<str>) { // 640
            description("Invalid marker Error"),
            display("Invalid marker Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        InvalidContextError(code: StatusCode, message: Box<str>) { // 701
            description("Invalid context Error"),
            display("Invalid context Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        UnknownClientError(code: StatusCode, message: Box<str>) { // Other
            description("Unknown client Error"),
            display("Unknown client Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
        UnknownServerError(code: StatusCode, message: Box<str>) { // Other
            description("Unknown server Error"),
            display("Unknown server Error: HTTP Status Code = {}, Error Message = {}", code, message),
        }
    }
}
